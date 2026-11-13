//! Tampilan PDF Catcher: satu jendela, tiga langkah bernomor.

pub mod actions;
pub mod basket;
pub mod catch_zone;
pub mod save_to;
pub mod settings_view;
pub mod theme;
pub mod widgets;

use egui::{Align2, Color32, RichText};

use crate::app::App;
use theme::*;

pub fn show(app: &mut App, ctx: &egui::Context) {
    // ---------- bilah atas ----------
    egui::TopBottomPanel::top("top")
        .frame(egui::Frame::default().fill(CARD).inner_margin(egui::Margin::symmetric(24, 12)).stroke(egui::Stroke::new(1.0_f32, BORDER)))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(34.0, 34.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 8.0, ACCENT);
                widgets::paint_page(ui, rect.shrink2(egui::vec2(10.0, 7.0)), Color32::WHITE, "");
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.label(RichText::new("PDF Catcher").font(semibold(20.0)));
                    ui.label(RichText::new("Tangkap file, lalu pilih mau diapakan.").color(MUTED).size(13.5));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::secondary_button(ui, "⚙  Pengaturan")
                        .on_hover_text("Atur di mana file hasil disimpan")
                        .clicked()
                    {
                        app.show_settings = true;
                    }
                });
            });
        });

    // ---------- isi ----------
    egui::CentralPanel::default()
        .frame(egui::Frame::default().fill(BG).inner_margin(egui::Margin::symmetric(16, 18)))
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                let avail = ui.available_width();
                let w = avail.min(960.0);
                ui.horizontal(|ui| {
                    ui.add_space(((avail - w) / 2.0).max(0.0));
                    ui.vertical(|ui| {
                        ui.set_width(w);
                        content(app, ui);
                        ui.add_space(24.0);
                    });
                });
            });
        });

    catch_zone::drag_overlay(ctx);
    show_toasts(app, ctx);
    settings_view::show(app, ctx);
}

fn content(app: &mut App, ui: &mut egui::Ui) {
    if app.engine_missing {
        widgets::message_box(
            ui,
            widgets::Tone::Err,
            "File pdfium.dll tidak ditemukan di folder aplikasi, jadi sebagian fitur PDF belum bisa diproses. Word/Excel/PowerPoint ke PDF tidak terpengaruh. Pasang ulang PDF Catcher untuk memperbaikinya.",
        );
        ui.add_space(8.0);
    }

    // Langkah 1
    widgets::card(ui, |ui| {
        let hint = if app.basket.is_empty() {
            "Seret PDF, Word, Excel, PowerPoint, atau JPG/PNG ke jendela ini, atau klik kotak di bawah."
        } else {
            "Bisa ditambah kapan saja. Seret ≡ untuk mengubah urutan."
        };
        widgets::step_header(ui, 1, "Tangkap file", hint, true);
        if app.basket.is_empty() {
            catch_zone::big_zone(app, ui);
        } else {
            basket::show(app, ui);
        }
    });
    ui.add_space(14.0);

    // Langkah 2
    let has_files = !app.basket.is_empty();
    widgets::card(ui, |ui| {
        widgets::step_header(ui, 2, "Pilih aksi", "Mau diapakan file-nya?", has_files);
        actions::cards(app, ui);
    });
    ui.add_space(14.0);

    // Langkah 3
    let step3 = has_files && app.action.is_some();
    widgets::card(ui, |ui| {
        let title = match app.action {
            Some(a) => format!("Atur & simpan: {}", a.title()),
            None => "Atur & simpan".to_string(),
        };
        widgets::step_header(ui, 3, &title, if step3 { "" } else { "Pilih aksi di langkah 2 dulu." }, step3);
        if step3 || !matches!(app.status, crate::app::Status::Idle) {
            actions::panel(app, ui);
        }
    });
}

fn show_toasts(app: &App, ctx: &egui::Context) {
    if app.toasts.is_empty() {
        return;
    }
    egui::Area::new(egui::Id::new("toasts"))
        .anchor(Align2::CENTER_TOP, egui::vec2(0.0, 76.0))
        .order(egui::Order::Tooltip)
        .interactable(false)
        .show(ctx, |ui| {
            for t in &app.toasts {
                let age = t.born.elapsed().as_secs_f32();
                let alpha = if age > 3.6 { ((4.2 - age) / 0.6).clamp(0.0, 1.0) } else { 1.0 };
                let (bg, fg) = t.tone.colors();
                egui::Frame::default()
                    .fill(bg.gamma_multiply(alpha))
                    .stroke(egui::Stroke::new(1.0_f32, fg.gamma_multiply(0.35 * alpha)))
                    .corner_radius(20.0)
                    .inner_margin(egui::Margin::symmetric(18, 9))
                    .shadow(egui::epaint::Shadow { offset: [0, 4], blur: 16, spread: 0, color: Color32::from_black_alpha((30.0 * alpha) as u8) })
                    .show(ui, |ui| {
                        ui.label(RichText::new(format!("{}  {}", t.tone.icon(), t.text)).color(fg.gamma_multiply(alpha)).size(15.5));
                    });
                ui.add_space(6.0);
            }
        });
}
