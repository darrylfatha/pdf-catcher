//! Zona tangkap besar (saat keranjang kosong) dan lapisan sorot saat file diseret.

use egui::{pos2, vec2, Align2, Color32, Sense, Shape, Stroke};

use super::theme::{self, *};
use super::widgets;
use crate::app::App;

fn dashed_rect(painter: &egui::Painter, r: egui::Rect, stroke: Stroke) {
    let c = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
    painter.extend(Shape::dashed_line(&c, stroke, 10.0, 7.0));
}

/// Kotak besar "Seret PDF ke sini". Diklik = membuka dialog pilih file.
pub fn big_zone(app: &mut App, ui: &mut egui::Ui) {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 230.0), Sense::click());
    let hovered = resp.hovered();
    let p = ui.painter();
    p.rect_filled(rect, 14.0, if hovered { ACCENT_SOFT } else { Color32::from_rgb(0xFA, 0xFA, 0xFB) });
    dashed_rect(p, rect.shrink(1.0), Stroke::new(2.0_f32, if hovered { ACCENT } else { DISABLED }));

    // ikon: lembar file jatuh ke lengkung "tangkap"
    let c = rect.center() - vec2(0.0, 42.0);
    widgets::paint_page(ui, egui::Rect::from_center_size(c, vec2(44.0, 56.0)), ACCENT, "");
    let arc: Vec<egui::Pos2> = (0..=20)
        .map(|i| {
            let t = i as f32 / 20.0 * std::f32::consts::PI;
            pos2(c.x - 40.0 * t.cos(), c.y + 22.0 + 22.0 * t.sin())
        })
        .collect();
    p.add(Shape::line(arc, Stroke::new(4.0_f32, Color32::from_rgb(0xF2, 0xB6, 0x1F))));

    p.text(rect.center() + vec2(0.0, 38.0), Align2::CENTER_CENTER, "Seret file ke sini", theme::semibold(21.0), TEXT);
    p.text(
        rect.center() + vec2(0.0, 66.0),
        Align2::CENTER_CENTER,
        "PDF, Word, Excel, PowerPoint, atau JPG/PNG — atau klik untuk memilih, bisa juga satu folder",
        egui::FontId::proportional(14.5),
        MUTED,
    );
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Klik untuk memilih file (Ctrl+O)");
    if resp.clicked() {
        app.pick_files();
    }
}

/// Lapisan sorot di atas seluruh jendela selama file sedang diseret.
pub fn drag_overlay(ctx: &egui::Context) {
    let n = ctx.input(|i| i.raw.hovered_files.len());
    if n == 0 {
        return;
    }
    let screen = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("drop-overlay")));
    painter.rect_filled(screen, 0.0, Color32::from_rgba_unmultiplied(0xD9, 0x35, 0x2B, 38));
    let inner = screen.shrink(18.0);
    painter.rect_filled(inner, 18.0, Color32::from_rgba_unmultiplied(255, 255, 255, 215));
    dashed_rect(&painter, inner, Stroke::new(3.0_f32, ACCENT));
    painter.text(inner.center() - vec2(0.0, 18.0), Align2::CENTER_CENTER, "Lepas untuk menangkap", theme::semibold(30.0), ACCENT_DARK);
    let sub = if n == 1 { "1 file".to_string() } else { format!("{n} file") };
    painter.text(inner.center() + vec2(0.0, 22.0), Align2::CENTER_CENTER, sub, egui::FontId::proportional(18.0), MUTED);
}
