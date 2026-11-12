//! Jendela Pengaturan: di mana file hasil disimpan.

use egui::RichText;

use super::theme::*;
use super::widgets;
use crate::app::App;
use pdf_catcher::settings::OutputMode;

pub fn show(app: &mut App, ctx: &egui::Context) {
    if !app.show_settings {
        return;
    }
    let mut open = true;
    let mut changed = false;
    egui::Window::new(RichText::new("Pengaturan").font(semibold(20.0)))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .default_width(520.0)
        .open(&mut open)
        .frame(egui::Frame::window(&ctx.style()).inner_margin(22.0))
        .show(ctx, |ui| {
            ui.set_width(520.0);
            ui.label(RichText::new("Di mana file hasil disimpan?").font(semibold(17.0)));
            ui.add_space(4.0);

            let mode = &mut app.settings.output_mode;
            let options = [
                (OutputMode::SameAsSource, "Sama dengan folder file PDF asli", "Disarankan. Hasil muncul di samping file aslinya."),
                (OutputMode::Custom, "Selalu di satu folder pilihan saya", "Semua hasil dikumpulkan di satu tempat."),
                (OutputMode::Ask, "Tanya setiap kali", "Jendela Simpan muncul setiap kali aksi dijalankan."),
            ];
            for (value, title, desc) in options {
                if ui.radio(*mode == value, RichText::new(title).size(16.0)).clicked() && *mode != value {
                    *mode = value;
                    changed = true;
                }
                ui.indent(title, |ui| {
                    ui.label(RichText::new(desc).color(MUTED).size(13.5));
                });
            }

            if app.settings.output_mode == OutputMode::Custom {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let text = app
                        .settings
                        .custom_dir
                        .as_ref()
                        .map(|d| d.display().to_string())
                        .unwrap_or_else(|| "Belum dipilih".into());
                    ui.add(egui::Label::new(RichText::new(&text).size(15.0)).truncate());
                    if widgets::secondary_button(ui, "Pilih folder…").clicked() {
                        if let Some(d) = rfd::FileDialog::new().set_title("Pilih folder hasil").pick_folder() {
                            app.settings.custom_dir = Some(d);
                            changed = true;
                        }
                    }
                });
                if app.settings.custom_dir.is_none() {
                    widgets::message_box(ui, widgets::Tone::Warn, "Pilih foldernya dulu. Sampai dipilih, hasil disimpan di folder file asli.");
                }
            }

            ui.add_space(12.0);
            ui.separator();
            ui.label(RichText::new(format!("File pengaturan: {}", app.config_path.display())).color(MUTED).size(12.5));
            ui.label(RichText::new(format!("PDF Catcher versi {}", env!("CARGO_PKG_VERSION"))).color(MUTED).size(12.5));
            ui.add_space(6.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::primary_button(ui, "Selesai", true, "").clicked() {
                    app.show_settings = false;
                }
            });
        });
    if changed {
        app.save_settings();
    }
    if !open {
        app.show_settings = false;
    }
}
