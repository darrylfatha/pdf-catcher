//! State aplikasi dan alur utama: tangkap -> pilih aksi -> simpan.

use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use pdf_catcher::core::intake::{self, FileInfo, FileStatus};
use pdf_catcher::core::output;
use pdf_catcher::core::ranges;
use pdf_catcher::core::split::{self, SplitMode};
use pdf_catcher::error::AppError;
use pdf_catcher::settings::{self, OutputMode, Settings};
use pdf_catcher::worker::{Engine, Event, Job};

use crate::ui::widgets::Tone;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Merge,
    Split,
    Convert,
}

impl Action {
    pub fn title(self) -> &'static str {
        match self {
            Action::Merge => "Gabung",
            Action::Split => "Pisah",
            Action::Convert => "Ke Word",
        }
    }
    pub fn run_label(self) -> &'static str {
        match self {
            Action::Merge => "Gabungkan sekarang",
            Action::Split => "Pisahkan sekarang",
            Action::Convert => "Ubah ke Word sekarang",
        }
    }
    pub fn busy_label(self) -> &'static str {
        match self {
            Action::Merge => "Sedang menggabungkan PDF…",
            Action::Split => "Sedang memisahkan PDF…",
            Action::Convert => "Sedang mengubah PDF ke Word…",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SplitChoice {
    EveryPage,
    EveryN,
    Custom,
}

pub struct Item {
    pub path: PathBuf,
    pub key: String,
    /// `None` selama file masih diperiksa.
    pub info: Option<FileInfo>,
}

impl Item {
    pub fn is_ready(&self) -> bool {
        matches!(self.info.as_ref().map(|i| i.status), Some(FileStatus::Ready))
    }
    pub fn name(&self) -> String {
        pdf_catcher::error::file_name(&self.path)
    }
}

pub enum Status {
    Idle,
    Running { action: Action, done: usize, total: usize },
    Done { action: Action, files: Vec<PathBuf> },
    Failed(String),
    Cancelled,
}

pub struct Toast {
    pub tone: Tone,
    pub text: String,
    pub born: Instant,
}

pub struct App {
    pub engine: Engine,
    pub engine_missing: bool,
    pub incoming: Receiver<Vec<PathBuf>>,

    pub settings: Settings,
    pub config_path: PathBuf,
    pub show_settings: bool,

    pub basket: Vec<Item>,
    pub action: Option<Action>,

    pub merge_name: String,
    pub split_choice: SplitChoice,
    pub split_n: usize,
    pub split_text: String,
    pub split_prefix: String,
    pub convert_all: bool,
    pub convert_text: String,
    pub convert_name: String,

    /// Folder tujuan khusus untuk pekerjaan ini saja (tombol "Ganti folder").
    pub dir_override: Option<PathBuf>,
    pub make_default: bool,

    pub status: Status,
    pub toasts: Vec<Toast>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext, initial: Vec<PathBuf>, incoming: Receiver<Vec<PathBuf>>) -> Self {
        crate::ui::theme::install(&cc.egui_ctx);
        let ctx = cc.egui_ctx.clone();
        let engine = Engine::start(move || ctx.request_repaint());
        let config_path = settings::config_path();
        let mut app = App {
            engine,
            engine_missing: false,
            incoming,
            settings: Settings::load(&config_path),
            config_path,
            show_settings: false,
            basket: Vec::new(),
            action: None,
            merge_name: String::new(),
            split_choice: SplitChoice::EveryPage,
            split_n: 2,
            split_text: String::new(),
            split_prefix: String::new(),
            convert_all: true,
            convert_text: String::new(),
            convert_name: String::new(),
            dir_override: None,
            make_default: false,
            status: Status::Idle,
            toasts: Vec::new(),
        };
        if !initial.is_empty() {
            app.catch(initial);
        }
        app
    }

    // ---------- tangkap ----------

    pub fn toast(&mut self, tone: Tone, text: impl Into<String>) {
        self.toasts.push(Toast { tone, text: text.into(), born: Instant::now() });
    }

    /// Menangkap file/folder yang dilepas atau dipilih.
    pub fn catch(&mut self, paths: Vec<PathBuf>) {
        if paths.is_empty() {
            return;
        }
        let collected = intake::collect(&paths);
        let mut added = Vec::new();
        let mut dup = 0;
        for p in collected.pdfs {
            let key = intake::same_file_key(&p);
            if self.basket.iter().any(|i| i.key == key) {
                dup += 1;
                continue;
            }
            self.basket.push(Item { path: p.clone(), key, info: None });
            added.push(p);
        }
        if !added.is_empty() {
            self.toast(Tone::Ok, format!("{} PDF tertangkap", added.len()));
            self.engine.inspect(added);
            if matches!(self.status, Status::Done { .. } | Status::Failed(_) | Status::Cancelled) {
                self.status = Status::Idle;
            }
        }
        if collected.skipped > 0 {
            self.toast(Tone::Warn, format!("{} file dilewati karena bukan PDF", collected.skipped));
        }
        if dup > 0 {
            self.toast(Tone::Info, format!("{dup} file sudah ada di keranjang"));
        }
        if self.basket.is_empty() && collected.skipped == 0 && dup == 0 {
            self.toast(Tone::Warn, "Tidak ada file PDF yang ditemukan");
        }
    }

    pub fn pick_files(&mut self) {
        if let Some(files) = rfd::FileDialog::new()
            .set_title("Pilih file PDF")
            .add_filter("File PDF", &["pdf", "PDF"])
            .pick_files()
        {
            self.catch(files);
        }
    }

    pub fn clear_basket(&mut self) {
        self.basket.clear();
        self.action = None;
        self.dir_override = None;
        self.make_default = false;
        self.merge_name.clear();
        self.split_prefix.clear();
        self.convert_name.clear();
        self.split_text.clear();
        self.convert_text.clear();
        if !self.is_running() {
            self.status = Status::Idle;
        }
    }

    pub fn ready_items(&self) -> Vec<&Item> {
        self.basket.iter().filter(|i| i.is_ready()).collect()
    }

    pub fn inspecting(&self) -> bool {
        self.basket.iter().any(|i| i.info.is_none())
    }

    pub fn is_running(&self) -> bool {
        matches!(self.status, Status::Running { .. })
    }

    // ---------- event dari thread pekerja dan dari jendela lain ----------

    pub fn poll(&mut self, ctx: &egui::Context) {
        while let Ok(ev) = self.engine.rx.try_recv() {
            match ev {
                Event::EngineMissing => self.engine_missing = true,
                Event::Inspected(infos) => {
                    for info in infos {
                        if let Some(item) = self.basket.iter_mut().find(|i| i.path == info.path) {
                            item.info = Some(info);
                        }
                    }
                }
                Event::Progress { done, total } => {
                    if let Status::Running { done: d, total: t, .. } = &mut self.status {
                        *d = done;
                        *t = total;
                    }
                }
                Event::Done(files) => {
                    let action = match self.status {
                        Status::Running { action, .. } => action,
                        _ => Action::Merge,
                    };
                    self.status = Status::Done { action, files };
                }
                Event::Failed(AppError::Cancelled) => self.status = Status::Cancelled,
                Event::Failed(e) => self.status = Status::Failed(e.to_string()),
            }
        }
        let mut got = Vec::new();
        let mut pinged = false;
        while let Ok(paths) = self.incoming.try_recv() {
            pinged = true;
            got.extend(paths);
        }
        if pinged {
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            self.catch(got);
        }
        self.toasts.retain(|t| t.born.elapsed() < Duration::from_millis(4200));
        if !self.toasts.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(200));
        }
    }

    // ---------- ketersediaan aksi ----------

    /// `Ok(())` jika aksi bisa dipilih, atau alasan singkat jika tidak.
    pub fn availability(&self, a: Action) -> Result<(), &'static str> {
        let ready = self.ready_items().len();
        match a {
            Action::Merge if ready < 2 => Err("Butuh minimal 2 PDF di keranjang"),
            Action::Split | Action::Convert if ready == 0 => Err("Tangkap 1 PDF dulu"),
            Action::Split | Action::Convert if ready > 1 => Err("Hanya untuk 1 PDF. Keluarkan PDF lainnya dari keranjang"),
            _ => Ok(()),
        }
    }

    pub fn single(&self) -> Option<&FileInfo> {
        let ready = self.ready_items();
        if ready.len() == 1 {
            ready[0].info.as_ref()
        } else {
            None
        }
    }

    /// Mode pisah dari pilihan pengguna, atau pesan kesalahan.
    pub fn split_mode(&self) -> Result<SplitMode, String> {
        let pages = self.single().map(|i| i.pages).unwrap_or(0);
        match self.split_choice {
            SplitChoice::EveryPage => Ok(SplitMode::EveryPage),
            SplitChoice::EveryN => Ok(SplitMode::EveryN(self.split_n.max(1))),
            SplitChoice::Custom => {
                if self.split_text.trim().is_empty() {
                    return Err("Ketik halaman yang ingin diambil, misalnya 1-3, 5".into());
                }
                ranges::parse_ranges(&self.split_text, pages)
                    .map(SplitMode::Ranges)
                    .map_err(|_| format!("Penulisan halaman belum benar. Contoh: 1-3, 5. PDF ini punya {pages} halaman."))
            }
        }
    }

    pub fn split_count(&self) -> Option<usize> {
        let pages = self.single()?.pages;
        self.split_mode().ok().map(|m| split::plan(&m, pages).len())
    }

    pub fn convert_pages(&self) -> Result<Option<Vec<std::ops::RangeInclusive<usize>>>, String> {
        if self.convert_all {
            return Ok(None);
        }
        let pages = self.single().map(|i| i.pages).unwrap_or(0);
        if self.convert_text.trim().is_empty() {
            return Err("Ketik halaman yang ingin diubah, misalnya 1-3, 5".into());
        }
        ranges::parse_ranges(&self.convert_text, pages)
            .map(Some)
            .map_err(|_| format!("Penulisan halaman belum benar. Contoh: 1-3, 5. PDF ini punya {pages} halaman."))
    }

    /// Alasan tombol utama belum bisa ditekan, atau `None` jika siap.
    pub fn blocker(&self) -> Option<String> {
        if self.engine_missing {
            return Some("pdfium.dll tidak ditemukan".into());
        }
        if self.inspecting() {
            return Some("Tunggu sebentar, file sedang diperiksa".into());
        }
        let a = self.action?;
        if let Err(why) = self.availability(a) {
            return Some(why.into());
        }
        match a {
            Action::Split => self.split_mode().err(),
            Action::Convert => self.convert_pages().err(),
            Action::Merge => None,
        }
    }

    // ---------- folder hasil ----------

    pub fn source_dir(&self) -> Option<PathBuf> {
        self.ready_items().first().and_then(|i| i.path.parent().map(|p| p.to_path_buf()))
    }

    /// Folder tujuan yang akan dipakai. `None` berarti akan ditanyakan saat menyimpan.
    pub fn target_dir(&self) -> Option<PathBuf> {
        if let Some(d) = &self.dir_override {
            return Some(d.clone());
        }
        match self.settings.output_mode {
            OutputMode::SameAsSource => self.source_dir(),
            OutputMode::Custom => self.settings.custom_dir.clone().or_else(|| self.source_dir()),
            OutputMode::Ask => None,
        }
    }

    pub fn save_settings(&mut self) {
        if let Err(e) = self.settings.save(&self.config_path) {
            self.toast(Tone::Err, format!("Pengaturan gagal disimpan: {e}"));
        }
    }

    pub fn default_name(&self, a: Action) -> String {
        match a {
            Action::Merge => "gabungan".into(),
            Action::Split | Action::Convert => self
                .ready_items()
                .first()
                .map(|i| output::stem_of(&i.path))
                .unwrap_or_else(|| "hasil".into()),
        }
    }

    pub fn name_field(&mut self, a: Action) -> &mut String {
        match a {
            Action::Merge => &mut self.merge_name,
            Action::Split => &mut self.split_prefix,
            Action::Convert => &mut self.convert_name,
        }
    }

    // ---------- jalankan ----------

    pub fn start(&mut self) {
        let Some(action) = self.action else { return };
        if self.blocker().is_some() {
            return;
        }
        let default = self.default_name(action);
        let mut name = output::sanitize_stem(self.name_field(action).as_str(), &default);

        let dir = match self.target_dir() {
            Some(d) => d,
            None => {
                // Mode "Tanya setiap kali".
                let start_dir = self.source_dir().unwrap_or_default();
                match action {
                    Action::Split => match rfd::FileDialog::new()
                        .set_title("Pilih folder untuk menyimpan hasil")
                        .set_directory(&start_dir)
                        .pick_folder()
                    {
                        Some(d) => d,
                        None => return,
                    },
                    Action::Merge | Action::Convert => {
                        let ext = if action == Action::Merge { "pdf" } else { "docx" };
                        let label = if action == Action::Merge { "File PDF" } else { "Dokumen Word" };
                        match rfd::FileDialog::new()
                            .set_title("Simpan hasil sebagai")
                            .set_directory(&start_dir)
                            .set_file_name(format!("{name}.{ext}"))
                            .add_filter(label, &[ext])
                            .save_file()
                        {
                            Some(p) => {
                                name = output::sanitize_stem(&output::stem_of(&p), &default);
                                match p.parent() {
                                    Some(d) => d.to_path_buf(),
                                    None => return,
                                }
                            }
                            None => return,
                        }
                    }
                }
            }
        };

        if let Err(e) = output::check_writable(&dir) {
            self.status = Status::Failed(e.to_string());
            return;
        }
        if self.make_default && self.dir_override.is_some() {
            self.settings.output_mode = OutputMode::Custom;
            self.settings.custom_dir = Some(dir.clone());
            self.save_settings();
            self.dir_override = None;
            self.make_default = false;
            self.toast(Tone::Ok, "Folder ini sekarang jadi tujuan default");
        }

        let ready: Vec<PathBuf> = self.ready_items().iter().map(|i| i.path.clone()).collect();
        let job = match action {
            Action::Merge => Job::Merge { inputs: ready, out_dir: dir, name },
            Action::Split => match self.split_mode() {
                Ok(mode) => Job::Split { input: ready[0].clone(), mode, out_dir: dir, prefix: name },
                Err(_) => return,
            },
            Action::Convert => match self.convert_pages() {
                Ok(pages) => Job::Convert { input: ready[0].clone(), pages, out_dir: dir, name },
                Err(_) => return,
            },
        };
        self.status = Status::Running { action, done: 0, total: 1 };
        self.engine.run(job);
    }

    pub fn cancel(&self) {
        self.engine.cancel();
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll(ctx);

        // File yang dilepas ke jendela.
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        if !dropped.is_empty() {
            self.catch(dropped);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::O)) {
            self.pick_files();
        }

        crate::ui::show(self, ctx);
    }
}
