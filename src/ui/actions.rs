//! Langkah 2 (kartu aksi) dan langkah 3 (opsi, simpan, proses, hasil).

use egui::{pos2, vec2, Align2, Color32, Rect, RichText, Sense, Stroke};

use super::theme::{self, *};
use super::widgets::{self, Tone};
use super::save_to;
use crate::app::{Action, App, SplitChoice, Status};

const WORD_BLUE: Color32 = Color32::from_rgb(0x2B, 0x57, 0x9A);

fn paint_icon(ui: &egui::Ui, a: Action, rect: Rect, enabled: bool) {
    let red = if enabled { ACCENT } else { DISABLED };
    let o = rect.left_top();
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
            let blue = if enabled { WORD_BLUE } else { DISABLED };
            widgets::paint_page(ui, Rect::from_min_size(o + vec2(0.0, 6.0), vec2(26.0, 34.0)), red.gamma_multiply(0.6), "");
            widgets::paint_page(ui, Rect::from_min_size(o + vec2(18.0, 12.0), vec2(30.0, 38.0)), blue, "W");
        }
    }
}

fn description(a: Action) -> &'static str {
    match a {
        Action::Merge => "Satukan beberapa PDF menjadi satu file.",
        Action::Split => "Pecah satu PDF menjadi beberapa file.",
        Action::Convert => "Ubah PDF menjadi dokumen Word yang bisa diedit.",
    }
}

/// Tiga kartu aksi besar yang bisa diklik.
pub fn cards(app: &mut App, ui: &mut egui::Ui) {
    let running = app.is_running();
    // Pilihan yang tidak lagi cocok dengan isi keranjang dilepas.
    if let Some(a) = app.action {
        if app.availability(a).is_err() && !app.inspecting() && !running {
            app.action = None;
        }
    }

    let gap = 12.0;
    let w = ((ui.available_width() - 2.0 * gap) / 3.0).max(150.0);
    let h = 150.0;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for a in [Action::Merge, Action::Split, Action::Convert] {
            let avail = if app.basket.is_empty() { Err("Tangkap PDF dulu di langkah 1") } else { app.availability(a) };
            let enabled = avail.is_ok() && !running && !app.engine_missing;
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

            let icon = Rect::from_min_size(rect.left_top() + vec2(16.0, 14.0), vec2(52.0, 52.0));
            paint_icon(ui, a, icon, enabled);
            if selected {
                let c = pos2(rect.right() - 20.0, rect.top() + 20.0);
                p.circle_filled(c, 11.0, ACCENT);
                p.text(c, Align2::CENTER_CENTER, "✔", egui::FontId::proportional(13.0), Color32::WHITE);
            }

            let text_color = if enabled { TEXT } else { DISABLED };
            p.text(rect.left_top() + vec2(16.0, 76.0), Align2::LEFT_TOP, a.title(), theme::semibold(19.0), text_color);
            let (sub, sub_color) = match avail {
                Ok(()) => (description(a), MUTED),
                Err(why) => (why, if app.basket.is_empty() { DISABLED } else { WARN }),
            };
            let galley = ui.painter().layout(sub.to_string(), egui::FontId::proportional(13.5), sub_color, w - 32.0);
            ui.painter().galley(rect.left_top() + vec2(16.0, 104.0), galley, sub_color);

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
    });
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
                    .on_hover_text(format!("Kembali ke pengaturan {} dengan PDF yang sama", action.title()))
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
            let n = app.ready_items().len();
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
    }
}

/// Membuka Explorer dengan file hasil langsung tersorot.
pub fn open_in_explorer(path: &std::path::Path) {
    use std::os::windows::process::CommandExt;
    let _ = std::process::Command::new("explorer.exe")
        .raw_arg(format!("/select,\"{}\"", path.display()))
        .spawn();
}
