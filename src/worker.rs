//! Thread pekerja. Pdfium tidak thread-safe, jadi SATU thread memegang satu instance
//! Pdfium selama aplikasi hidup dan mengerjakan perintah satu per satu:
//! memeriksa file yang baru ditangkap, dan menjalankan pekerjaan (gabung/pisah/konversi).

use std::ops::RangeInclusive;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

use crate::core::intake::{self, FileInfo};
use crate::core::split::SplitMode;
use crate::core::{self, convert, merge, output, split, Reporter};
use crate::error::AppError;

#[derive(Debug, Clone)]
pub enum Job {
    Merge { inputs: Vec<PathBuf>, out_dir: PathBuf, name: String },
    Split { input: PathBuf, mode: SplitMode, out_dir: PathBuf, prefix: String },
    Convert { input: PathBuf, pages: Option<Vec<RangeInclusive<usize>>>, out_dir: PathBuf, name: String },
}

enum Command {
    Inspect(Vec<PathBuf>),
    Run(Job),
}

#[derive(Debug)]
pub enum Event {
    /// Pdfium gagal dimuat; aplikasi tidak bisa memproses apa pun.
    EngineMissing,
    Inspected(Vec<FileInfo>),
    Progress { done: usize, total: usize },
    Done(Vec<PathBuf>),
    Failed(AppError),
}

pub struct Engine {
    tx: Sender<Command>,
    pub rx: Receiver<Event>,
    cancel: Arc<AtomicBool>,
}

impl Engine {
    /// Menyalakan thread pekerja. `wake` dipanggil setiap kali ada event baru
    /// (dipakai UI untuk menggambar ulang).
    pub fn start(wake: impl Fn() + Send + 'static) -> Engine {
        let (tx, cmd_rx) = channel::<Command>();
        let (ev_tx, rx) = channel::<Event>();
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_w = cancel.clone();

        std::thread::Builder::new()
            .name("pdfcatcher-worker".into())
            .spawn(move || {
                let send = |e: Event| {
                    let _ = ev_tx.send(e);
                    wake();
                };
                let pdfium = match core::load_pdfium(&core::exe_dir()) {
                    Ok(p) => Some(p),
                    Err(_) => {
                        send(Event::EngineMissing);
                        None
                    }
                };
                while let Ok(cmd) = cmd_rx.recv() {
                    let Some(pdfium) = pdfium.as_ref() else {
                        if matches!(cmd, Command::Run(_)) {
                            send(Event::Failed(AppError::PdfiumMissing));
                        }
                        continue;
                    };
                    match cmd {
                        Command::Inspect(paths) => {
                            let infos = paths.iter().map(|p| intake::inspect(pdfium, p)).collect();
                            send(Event::Inspected(infos));
                        }
                        Command::Run(job) => {
                            cancel_w.store(false, Ordering::Relaxed);
                            let mut progress = |done, total| send(Event::Progress { done, total });
                            let mut rep = Reporter::new(&cancel_w, &mut progress);
                            let result = run(pdfium, job, &mut rep);
                            drop(rep);
                            match result {
                                Ok(paths) => send(Event::Done(paths)),
                                Err(e) => send(Event::Failed(e)),
                            }
                        }
                    }
                }
            })
            .expect("thread pekerja");

        Engine { tx, rx, cancel }
    }

    pub fn inspect(&self, paths: Vec<PathBuf>) {
        let _ = self.tx.send(Command::Inspect(paths));
    }

    pub fn run(&self, job: Job) {
        self.cancel.store(false, Ordering::Relaxed);
        let _ = self.tx.send(Command::Run(job));
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

fn run(pdfium: &pdfium_render::prelude::Pdfium, job: Job, rep: &mut Reporter) -> Result<Vec<PathBuf>, AppError> {
    match job {
        Job::Merge { inputs, out_dir, name } => {
            output::check_writable(&out_dir)?;
            let target = output::unique_path(&out_dir, &name, "pdf", &[]);
            merge::merge(pdfium, &inputs, &target, rep).map(|p| vec![p])
        }
        Job::Split { input, mode, out_dir, prefix } => {
            output::check_writable(&out_dir)?;
            split::split(pdfium, &input, &mode, &out_dir, &prefix, rep)
        }
        Job::Convert { input, pages, out_dir, name } => {
            output::check_writable(&out_dir)?;
            let target = output::unique_path(&out_dir, &name, "docx", &[]);
            convert::pdf_to_docx(pdfium, &input, pages.as_deref(), &target, rep).map(|p| vec![p])
        }
    }
}
