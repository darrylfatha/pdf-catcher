//! Komponen kecil yang dipakai berulang: kartu, judul langkah, tombol utama, kotak pesan.

use egui::{vec2, Align2, Color32, Frame, Rect, Response, RichText, Sense, Stroke, Ui};

use super::theme::{self, *};

/// Kartu putih bersudut bulat.
pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::default()
        .fill(CARD)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(14.0)
        .inner_margin(20.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

/// Judul langkah bernomor: lingkaran angka + judul + petunjuk singkat.
pub fn step_header(ui: &mut Ui, number: usize, title: &str, hint: &str, active: bool) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::hover());
        let fill = if active { ACCENT } else { DISABLED };
        ui.painter().circle_filled(rect.center(), 17.0, fill);
        ui.painter().text(rect.center(), Align2::CENTER_CENTER, number.to_string(), theme::semibold(17.0), Color32::WHITE);
        ui.add_space(4.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            let color = if active { TEXT } else { MUTED };
            ui.label(RichText::new(title).font(theme::semibold(19.0)).color(color));
            if !hint.is_empty() {
                ui.label(RichText::new(hint).color(MUTED).size(14.5));
            }
        });
    });
    ui.add_space(6.0);
}

/// Tombol utama besar berwarna merah. Saat tidak aktif, `why_disabled` muncul saat kursor diarahkan.
pub fn primary_button(ui: &mut Ui, text: &str, enabled: bool, why_disabled: &str) -> Response {
    let btn = egui::Button::new(RichText::new(text).font(theme::semibold(17.0)).color(Color32::WHITE))
        .fill(if enabled { ACCENT } else { DISABLED })
        .stroke(Stroke::NONE)
        .corner_radius(10.0)
        .min_size(vec2(240.0, 48.0));
    let r = ui.add_enabled(enabled, btn);
    let r = if enabled { r.on_hover_cursor(egui::CursorIcon::PointingHand) } else { r };
    if !enabled && !why_disabled.is_empty() {
        r.on_disabled_hover_text(why_disabled)
    } else {
        r
    }
}

/// Tombol biasa dengan ukuran nyaman diklik.
pub fn secondary_button(ui: &mut Ui, text: &str) -> Response {
    ui.add(egui::Button::new(RichText::new(text).size(15.5)).min_size(vec2(0.0, 40.0)).corner_radius(10.0))
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Ok,
    Warn,
    Err,
    Info,
}

impl Tone {
    pub fn colors(self) -> (Color32, Color32) {
        match self {
            Tone::Ok => (OK_SOFT, OK),
            Tone::Warn => (WARN_SOFT, WARN),
            Tone::Err => (ERR_SOFT, ERR),
            Tone::Info => (Color32::from_rgb(0xEC, 0xF1, 0xFB), Color32::from_rgb(0x1F, 0x4E, 0xA8)),
        }
    }
    pub fn icon(self) -> &'static str {
        match self {
            Tone::Ok => "✔",
            Tone::Warn => "⚠",
            Tone::Err => "✖",
            Tone::Info => "ℹ",
        }
    }
}

/// Kotak pesan berwarna sesuai nada (berhasil, peringatan, galat, info).
pub fn message_box(ui: &mut Ui, tone: Tone, text: &str) {
    let (bg, fg) = tone.colors();
    Frame::default().fill(bg).corner_radius(10.0).inner_margin(egui::Margin::symmetric(14, 10)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(tone.icon()).color(fg).size(17.0));
            ui.label(RichText::new(text).color(fg).size(15.0));
        });
    });
}

/// Ikon kecil berbentuk lembar kertas dengan label (dipakai di kartu aksi dan keranjang).
pub fn paint_page(ui: &Ui, rect: Rect, color: Color32, label: &str) {
    let p = ui.painter();
    let fold = rect.width() * 0.3;
    let pts = vec![
        rect.left_top(),
        egui::pos2(rect.right() - fold, rect.top()),
        egui::pos2(rect.right(), rect.top() + fold),
        rect.right_bottom(),
        rect.left_bottom(),
    ];
    p.add(egui::Shape::convex_polygon(pts, color, Stroke::NONE));
    p.add(egui::Shape::convex_polygon(
        vec![
            egui::pos2(rect.right() - fold, rect.top()),
            egui::pos2(rect.right(), rect.top() + fold),
            egui::pos2(rect.right() - fold, rect.top() + fold),
        ],
        Color32::from_white_alpha(110),
        Stroke::NONE,
    ));
    if !label.is_empty() {
        p.text(
            egui::pos2(rect.center().x, rect.center().y + rect.height() * 0.1),
            Align2::CENTER_CENTER,
            label,
            theme::semibold(rect.width() * 0.34),
            Color32::WHITE,
        );
    }
}

/// Ukuran file yang mudah dibaca: "1,2 MB".
pub fn human_size(bytes: u64) -> String {
    let b = bytes as f64;
    let (v, unit) = if b >= 1024.0 * 1024.0 * 1024.0 {
        (b / (1024.0 * 1024.0 * 1024.0), "GB")
    } else if b >= 1024.0 * 1024.0 {
        (b / (1024.0 * 1024.0), "MB")
    } else if b >= 1024.0 {
        (b / 1024.0, "KB")
    } else {
        return format!("{bytes} B");
    };
    format!("{v:.1} {unit}").replace('.', ",")
}
