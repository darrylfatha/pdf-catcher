//! Langkah 2 (kartu aksi, dua baris: dari PDF & ke PDF) dan langkah 3 (opsi, simpan, proses, hasil).

use egui::{pos2, vec2, Align2, Color32, Rect, RichText, Sense, Stroke};

use super::save_to;
use super::theme::{self, *};
use super::widgets::{self, Tone};
use crate::app::{Action, App, SplitChoice, Status, FROM_PDF, TO_PDF};

const WORD_BLUE: Color32 = Color32::from_rgb(0x2B, 0x57, 0x9A);
const EXCEL_GREEN: Color32 = Color32::from_rgb(0x1D, 0x6F, 0x42);
const PPT_ORANGE: Color32 = Color32::from_rgb(0xC4, 0x3E, 0x1C);
const IMAGE_PURPLE: Color32 = Color32::from_rgb(0x6B, 0x4B, 0xC9);

fn paint_icon(ui: &egui::Ui, a: Action, rect: Rect, enabled: bool) {
    let red = if enabled { ACCENT } else { DISABLED };
    let o = rect.left_top();
    let accent_of = |c: Color32| if enabled { c } else { DISABLED };
    match a {
        Action::Merge => {
            widgets::paint_page(ui, Rect::from_min_size(o + vec2(2.0, 2.0), vec2(26.0, 34.0)), red.gamma_multiply(0.55), "");
            widgets::paint_page(ui, Rect::from_min_size(o + vec2(20.0, 14.0), vec2(28.0, 36.0)), red, "1");
        }
        Action::Split => {
            widgets::paint_page(ui, Rect::from_min_size(o + vec2(0.0, 8.0), vec2(28.0, 36.0)), red, "");
            widgets::paint_page(ui, Rect::from_min_size(o + vec2(34.0, 2.0), vec2(16.0, 21.0)), red.gamma_multiply(0.75), "");
            widgets::paint_page(ui, Rect::from_min_size(o + vec2(34.0, 28.0), vec2(16.0, 21.0)), red.gamma_multiply(0.75), "");
        }
        Action::Convert => {
            let blue = accent_of(WORD_BLUE);
            widgets::paint_page(ui, Rect::from_min_size(o + vec2(0.0, 6.0), vec2(26.0, 34.0)), red.gamma_multiply(0.6), "");
            widgets::paint_page(ui, Rect::from_min_size(o + vec2(18.0, 12.0), vec2(30.0, 38.0)), blue, "W");
        }
        Action::ImagesToPdf => {
            let purple = accent_of(IMAGE_PURPLE);
            widgets::paint_page(ui, Rect::from_min_size(o + vec2(2.0, 2.0), vec2(26.0, 34.0)), purple.gamma_multiply(0.55), "");
            widgets::paint_page(ui, Rect::from_min_size(o + vec2(20.0, 14.0), vec2(28.0, 36.0)), red, "PDF");
        }
        Action::WordToPdf => {
            let blue = accent_of(WORD_BLUE);
            widgets::paint_page(ui, Rect::from_min_size(o + vec2(0.0, 6.0), vec2(26.0, 34.0)), blue, "W");
            widgets::paint_page(ui, Rect::from_min_size(o + vec2(18.0, 12.0), vec2(30.0, 38.0)), red, "PDF");
        }
        Action::ExcelToPdf => {
            let green = accent_of(EXCEL_GREEN);
            widgets::paint_page(ui, Rect::from_min_size(o + vec2(0.0, 6.0), vec2(26.0, 34.0)), green, "X");
            widgets::paint_page(ui, Rect::from_min_size(o + vec2(18.0, 12.0), vec2(30.0, 38.0)), red, "PDF");
        }
        Action::PptToPdf => {
            let orange = accent_of(PPT_ORANGE);
            widgets::paint_page(ui, Rect::from_min_size(o + vec2(0.0, 6.0), vec2(26.0, 34.0)), orange, "P");
            widgets::paint_page(ui, Rect::from_min_size(o + vec2(18.0, 12.0), vec2(30.0, 38.0)), red, "PDF");
        }
    }
}

fn description(a: Action) -> &'static str {
    match a {
        Action::Merge => "Satukan beberapa PDF menjadi satu file.",
        Action::Split => "Pecah satu PDF menjadi beberapa file.",
        Action::Convert => "Ubah PDF menjadi dokumen Word yang bisa diedit.",
        Action::ImagesToPdf => "Satukan beberapa JPG/PNG menjadi satu PDF.",
        Action::WordToPdf => "Ubah dokumen Word menjadi PDF (lewat Microsoft Word).",
        Action::ExcelToPdf => "Ubah spreadsheet Excel menjadi PDF (lewat Microsoft Excel).",
        Action::PptToPdf => "Ubah presentasi PowerPoint menjadi PDF (lewat Microsoft PowerPoint).",
    }
}

fn card(app: &mut App, ui: &mut egui::Ui, a: Action, w: f32, h: f32, running: bool) {
    let avail = if app.basket.is_empty() { Err("Tangkap file dulu di langkah 1") } else { app.availability(a) };
    let enabled = avail.is_ok() && !running;
    let selected = app.action == Some(a);
    let (rect, resp) = ui.allocate_exact_size(vec2(w, h), if enabled { Sense::click() } else { Sense::hover() });
    let hovered = enabled && resp.hovered();

    let (fill, stroke) = if selected {
        (ACCENT_SOFT, Stroke::new(2.5_f32, ACCENT))
    } else if hovered {
        (Color32::from_rgb(0xFF, 0xF8, 0xF7), Stroke::new(1.5_f32, Color32::from_rgb(0xEC, 0x9C, 0x96)))
    } else if enabled {
        (CARD, Stroke::new(1.0_f32, BORDER))
    } else {
        (Color32::from_rgb(0xF4, 0xF4, 0xF6), Stroke::new(1.0_f32, BORDER))
    };
    let p = ui.painter();
    p.rect_filled(rect, 14.0, fill);
    p.rect_stroke(rect, 14.0, stroke, egui::StrokeKind::Inside);

    let icon = Rect::from_min_size(rect.left_top() + vec2(14.0, 12.0), vec2(46.0, 46.0));
    paint_icon(ui, a, icon, enabled);
    if selected {
        let c = pos2(rect.right() - 18.0, rect.top() + 18.0);
        p.circle_filled(c, 10.0, ACCENT);
        p.text(c, Align2::CENTER_CENTER, "✔", egui::FontId::proportional(12.0), Color32::WHITE);
    }

    let text_color = if enabled { TEXT } else { DISABLED };
    p.text(rect.left_top() + vec2(14.0, 64.0), Align2::LEFT_TOP, a.title(), theme::semibold(17.0), text_color);
    let (sub, sub_color) = match avail {
        Ok(()) => (description(a), MUTED),
        Err(why) => (why, if app.basket.is_empty() { DISABLED } else { WARN }),
    };
    let galley = ui.painter().layout(sub.to_string(), egui::FontId::proportional(13.0), sub_color, w - 28.0);
    ui.painter().galley(rect.left_top() + vec2(14.0, 88.0), galley, sub_color);

    let resp = if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(description(a))
    } else {
        resp
    };
    if resp.clicked() {
        app.action = Some(a);
        if matches!(app.status, Status::Done { .. } | Status::Failed(_) | Status::Cancelled) {
            app.status = Status::Idle;
        }
    }
}

fn row(app: &mut App, ui: &mut egui::Ui, actions: &[Action], running: bool) {
    let n = actions.len() as f32;
    let gap = 10.0;
    let w = ((ui.available_width() - (n - 1.0) * gap) / n).max(140.0);
    let h = 132.0;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for &a in actions {
            card(app, ui, a, w, h, running);
        }
    });
}

/// Kartu aksi, dalam dua baris: "Dari PDF" dan "Ke PDF".
pub fn cards(app: &mut App, ui: &mut egui::Ui) {
    let running = app.is_running();
    // Pilihan yang tidak lagi cocok dengan isi keranjang dilepas.
    if let Some(a) = app.action {
        if app.availability(a).is_err() && !app.inspecting() && !running {
            app.action = None;
        }
    }

    ui.label(RichText::new("Dari PDF").color(MUTED).size(13.0));
    ui.add_space(4.0);
    row(app, ui, &FROM_PDF, running);
    ui.add_space(12.0);
    ui.label(RichText::new("Ke PDF").color(MUTED).size(13.0));
    ui.add_space(4.0);
    row(app, ui, &TO_PDF, running);
}

/// Isi langkah 3: tergantung status (atur, sedang proses, selesai, gagal).
pub fn panel(app: &mut App, ui: &mut egui::Ui) {
    match &app.status {
        Status::Running { action, done, total } => {
            let (action, done, total) = (*action, *done, *total);
            ui.label(RichText::new(action.busy_label()).font(theme::semibold(17.0)));
            let frac = if total == 0 { 0.0 } else { done as f32 / total as f32 };
            ui.add(
                egui::ProgressBar::new(frac)
                    .desired_height(22.0)
                    .fill(ACCENT)
                    .text(RichText::new(format!("{done} dari {total}")).color(Color32::WHITE))
                    .animate(true),
            );
            ui.add_space(4.0);
            if widgets::secondary_button(ui, "Batal").on_hover_text("Hentikan proses. Tidak ada file setengah jadi yang tertinggal.").clicked() {
                app.cancel();
            }
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
        }
        Status::Done { action, files } => {
            let action = *action;
            let files = files.clone();
            let msg = if files.len() == 1 {
                "Selesai! 1 file berhasil disimpan.".to_string()
            } else {
                format!("Selesai! {} file berhasil disimpan.", files.len())
            };
            widgets::message_box(ui, Tone::Ok, &msg);
            ui.add_space(4.0);
            for f in files.iter().take(6) {
                ui.label(RichText::new(format!("•  {}", pdf_catcher::error::file_name(f))).size(15.0))
                    .on_hover_text(f.display().to_string());
            }
            if files.len() > 6 {
                ui.label(RichText::new(format!("…dan {} file lainnya", files.len() - 6)).color(MUTED));
            }
            if let Some(dir) = files.first().and_then(|f| f.parent()) {
                ui.label(RichText::new(format!("Lokasi: {}", dir.display())).color(MUTED).size(13.5));
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if widgets::primary_button(ui, "Buka folder hasil", true, "").clicked() {
                    if let Some(first) = files.first() {
                        open_in_explorer(first);
                    }
                }
                if widgets::secondary_button(ui, "Proses lagi")
                    .on_hover_text(format!("Kembali ke pengaturan {} dengan file yang sama", action.title()))
                    .clicked()
                {
                    app.status = Status::Idle;
                }
                if widgets::secondary_button(ui, "Mulai baru").on_hover_text("Kosongkan keranjang").clicked() {
                    app.clear_basket();
                }
            });
        }
        Status::Failed(msg) => {
            let msg = msg.clone();
            widgets::message_box(ui, Tone::Err, &msg);
            ui.add_space(6.0);
            if widgets::secondary_button(ui, "Mengerti, coba lagi").clicked() {
                app.status = Status::Idle;
            }
        }
        Status::Cancelled => {
            widgets::message_box(ui, Tone::Info, "Proses dibatalkan. Tidak ada file setengah jadi yang tertinggal.");
            ui.add_space(6.0);
            if widgets::secondary_button(ui, "Kembali").clicked() {
                app.status = Status::Idle;
            }
        }
        Status::Idle => {
            if let Some(a) = app.action {
                options(app, ui, a);
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);
                save_to::row(app, ui, a);
                ui.add_space(12.0);
                let blocker = app.blocker();
                ui.horizontal(|ui| {
                    if widgets::primary_button(ui, a.run_label(), blocker.is_none(), blocker.as_deref().unwrap_or("")).clicked() {
                        app.start();
                    }
                    if let Some(b) = &blocker {
                        ui.label(RichText::new(b).color(WARN).size(14.0));
                    }
                });
            }
        }
    }
}

fn big_radio<T: PartialEq + Copy>(ui: &mut egui::Ui, current: &mut T, value: T, text: &str) -> bool {
    let r = ui.radio(*current == value, RichText::new(text).size(16.0));
    if r.clicked() {
        *current = value;
        return true;
    }
    false
}

fn range_feedback(ui: &mut egui::Ui, result: Result<String, String>) {
    match result {
        Ok(s) => ui.label(RichText::new(format!("✔ {s}")).color(OK).size(14.0)),
        Err(e) => ui.label(RichText::new(format!("⚠ {e}")).color(WARN).size(14.0)),
    };
}

fn options(app: &mut App, ui: &mut egui::Ui, a: Action) {
    match a {
        Action::Merge => {
            let n = app.ready_items_of(a.source_kind()).len();
            widgets::message_box(
                ui,
                Tone::Info,
                &format!("{n} PDF akan disatukan sesuai urutan di keranjang. Ubah urutan dengan menyeret ≡ atau tombol ▲ ▼."),
            );
        }
        Action::Split => {
            let pages = app.single().map(|i| i.pages).unwrap_or(0);
            ui.label(RichText::new(format!("PDF ini punya {pages} halaman. Mau dipecah bagaimana?")).size(15.5));
            ui.add_space(2.0);
            big_radio(ui, &mut app.split_choice, SplitChoice::EveryPage, "Setiap halaman jadi 1 file");
            ui.horizontal(|ui| {
                big_radio(ui, &mut app.split_choice, SplitChoice::EveryN, "Setiap");
                let r = ui.add(egui::DragValue::new(&mut app.split_n).range(1..=pages.max(1)).speed(0.1));
                if r.changed() || r.gained_focus() {
                    app.split_choice = SplitChoice::EveryN;
                }
                ui.label(RichText::new("halaman jadi 1 file").size(16.0));
            });
            ui.horizontal(|ui| {
                big_radio(ui, &mut app.split_choice, SplitChoice::Custom, "Ambil halaman tertentu:");
                let r = ui.add(egui::TextEdit::singleline(&mut app.split_text).hint_text("contoh: 1-3, 5, 8-10").desired_width(220.0));
                if r.changed() || r.gained_focus() {
                    app.split_choice = SplitChoice::Custom;
                }
            });
            let feedback = match app.split_mode() {
                Ok(_) => Ok(match app.split_count() {
                    Some(1) => "Akan menghasilkan 1 file".to_string(),
                    Some(n) => format!("Akan menghasilkan {n} file"),
                    None => String::new(),
                }),
                Err(e) => Err(e),
            };
            range_feedback(ui, feedback);
        }
        Action::Convert => {
            let (pages, has_text) = app.single().map(|i| (i.pages, i.has_text)).unwrap_or((0, true));
            if !has_text {
                widgets::message_box(
                    ui,
                    Tone::Warn,
                    "PDF ini sepertinya hasil scan (berupa gambar). Teksnya kemungkinan tidak bisa diambil ke Word.",
                );
                ui.add_space(4.0);
            }
            ui.label(RichText::new("Halaman mana yang ingin diubah?").size(15.5));
            let mut all = app.convert_all;
            if ui.radio(all, RichText::new(format!("Semua halaman ({pages})")).size(16.0)).clicked() {
                all = true;
            }
            ui.horizontal(|ui| {
                if ui.radio(!all, RichText::new("Halaman tertentu:").size(16.0)).clicked() {
                    all = false;
                }
                let r = ui.add(egui::TextEdit::singleline(&mut app.convert_text).hint_text("contoh: 1-3, 5").desired_width(220.0));
                if r.changed() || r.gained_focus() {
                    all = false;
                }
            });
            app.convert_all = all;
            if !all {
                let fb = app.convert_pages().map(|p| {
                    let n = p.map(|r| pdf_catcher::core::ranges::page_list(&r).len()).unwrap_or(pages);
                    format!("{n} halaman akan diubah")
                });
                range_feedback(ui, fb);
            }
            ui.label(
                RichText::new("Hasil paling rapi untuk dokumen satu kolom seperti surat, laporan, dan kontrak.")
                    .color(MUTED)
                    .size(13.5),
            );
        }
        Action::ImagesToPdf => {
            let n = app.ready_items_of(a.source_kind()).len();
            widgets::message_box(
                ui,
                Tone::Info,
                &format!("{n} gambar akan disatukan menjadi satu PDF, satu gambar per halaman, sesuai urutan di keranjang."),
            );
        }
        Action::WordToPdf | Action::ExcelToPdf | Action::PptToPdf => {
            let n = app.ready_items_of(a.source_kind()).len();
            let app_name = match a {
                Action::WordToPdf => "Microsoft Word",
                Action::ExcelToPdf => "Microsoft Excel",
                Action::PptToPdf => "Microsoft PowerPoint",
                _ => unreachable!(),
            };
            let info = if n == 1 {
                format!("1 file akan diubah menjadi PDF lewat {app_name}, dengan nama yang sama.")
            } else {
                format!("{n} file akan diubah menjadi {n} PDF terpisah lewat {app_name}, masing-masing dengan nama yang sama seperti file aslinya.")
            };
            widgets::message_box(ui, Tone::Info, &info);
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!("Membutuhkan {app_name} terpasang di komputer ini. Akan berjalan sebentar di latar belakang, tidak terlihat."))
                    .color(MUTED)
                    .size(13.0),
            );
        }
    }
}

/// Membuka Explorer dengan file hasil langsung tersorot.
pub fn open_in_explorer(path: &std::path::Path) {
    use std::os::windows::process::CommandExt;
    let _ = std::process::Command::new("explorer.exe")
        .raw_arg(format!("/select,\"{}\"", path.display()))
        .spawn();
}
