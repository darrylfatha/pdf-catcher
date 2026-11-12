//! Keranjang tangkapan: daftar PDF yang sudah ditangkap.

use egui::{vec2, RichText, Sense, Stroke};

use super::theme::*;
use super::widgets;
use crate::app::App;
use pdf_catcher::core::intake::FileStatus;

const BIG_FILE: u64 = 200 * 1024 * 1024;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let running = app.is_running();
    let mut move_req: Option<(usize, usize)> = None;
    let mut remove: Option<usize> = None;
    let count = app.basket.len();

    {
        {
        for i in 0..count {
            let item = &app.basket[i];
            let row = egui::Frame::default()
                .fill(CARD)
                .stroke(Stroke::new(1.0_f32, BORDER))
                .corner_radius(10.0)
                .inner_margin(egui::Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        // pegangan seret
                        let handle = ui
                            .dnd_drag_source(egui::Id::new(("basket-row", i)), i, |ui| {
                                ui.label(RichText::new("≡").size(22.0).color(MUTED));
                            })
                            .response;
                        handle.on_hover_cursor(egui::CursorIcon::Grab).on_hover_text("Seret untuk mengubah urutan");

                        let (r, _) = ui.allocate_exact_size(vec2(26.0, 32.0), Sense::hover());
                        let color = match item.info.as_ref().map(|i| i.status) {
                            Some(FileStatus::Ready) | None => ACCENT,
                            _ => DISABLED,
                        };
                        widgets::paint_page(ui, r, color, "");

                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 1.0;
                            ui.label(RichText::new(item.name()).font(semibold(15.5)))
                                .on_hover_text(item.path.display().to_string());
                            match &item.info {
                                None => {
                                    ui.horizontal(|ui| {
                                        ui.add(egui::Spinner::new().size(13.0));
                                        ui.label(RichText::new("Memeriksa…").color(MUTED).size(13.5));
                                    });
                                }
                                Some(info) => match info.status {
                                    FileStatus::Ready => {
                                        let mut s = format!("{} halaman  ·  {}", info.pages, widgets::human_size(info.size));
                                        if info.size > BIG_FILE {
                                            s.push_str("  ·  file besar, mungkin agak lama");
                                        }
                                        ui.label(RichText::new(s).color(MUTED).size(13.5));
                                    }
                                    FileStatus::Encrypted => {
                                        ui.label(RichText::new("⚠ Berpassword, belum didukung. File ini tidak ikut diproses.").color(WARN).size(13.5));
                                    }
                                    FileStatus::Broken => {
                                        ui.label(RichText::new("✖ Rusak atau tidak bisa dibaca. File ini tidak ikut diproses.").color(ERR).size(13.5));
                                    }
                                },
                            }
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_enabled_ui(!running, |ui| {
                                if ui.button(RichText::new("✕").size(15.0)).on_hover_text("Keluarkan dari keranjang").clicked() {
                                    remove = Some(i);
                                }
                                if ui.add_enabled(i + 1 < count, egui::Button::new("▼")).on_hover_text("Turunkan").clicked() {
                                    move_req = Some((i, i + 2));
                                }
                                if ui.add_enabled(i > 0, egui::Button::new("▲")).on_hover_text("Naikkan").clicked() {
                                    move_req = Some((i, i));
                                }
                            });
                            ui.label(RichText::new(format!("{}", i + 1)).color(MUTED).size(14.0));
                        });
                    });
                });

            // seret-lepas di dalam daftar
            let resp = row.response;
            if let (Some(pointer), Some(_)) = (ui.input(|i| i.pointer.interact_pos()), resp.dnd_hover_payload::<usize>()) {
                let rect = resp.rect;
                let y = if pointer.y < rect.center().y { rect.top() - 3.0 } else { rect.bottom() + 3.0 };
                ui.painter().hline(rect.x_range(), y, Stroke::new(3.0_f32, ACCENT));
                if let Some(src) = resp.dnd_release_payload::<usize>() {
                    let to = if pointer.y < rect.center().y { i } else { i + 1 };
                    move_req = Some((*src, to));
                }
            }
            ui.add_space(2.0);
        }
        }
    }

    if !running {
        if let Some((from, to)) = move_req {
            if from < app.basket.len() {
                let item = app.basket.remove(from);
                let to = if to > from { to - 1 } else { to };
                app.basket.insert(to.min(app.basket.len()), item);
            }
        }
        if let Some(i) = remove {
            app.basket.remove(i);
            if app.basket.is_empty() {
                app.clear_basket();
            }
        }
    }

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_enabled_ui(!running, |ui| {
            if widgets::secondary_button(ui, "+  Tambah file").on_hover_text("Pilih PDF lain (Ctrl+O)").clicked() {
                app.pick_files();
            }
            if widgets::secondary_button(ui, "Kosongkan keranjang").clicked() {
                app.clear_basket();
            }
        });
        let ready = app.ready_items().len();
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(format!("{ready} PDF siap diproses")).color(MUTED));
        });
    });
}
