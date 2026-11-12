//! Warna, huruf, dan gaya dasar. Dibuat besar dan kontras agar nyaman untuk semua kalangan.

use std::sync::Arc;

use egui::{Color32, FontData, FontDefinitions, FontFamily, FontId, TextStyle};

pub const ACCENT: Color32 = Color32::from_rgb(0xD9, 0x35, 0x2B);
pub const ACCENT_DARK: Color32 = Color32::from_rgb(0xB5, 0x28, 0x1F);
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(0xFC, 0xEC, 0xEB);
pub const BG: Color32 = Color32::from_rgb(0xF5, 0xF5, 0xF7);
pub const CARD: Color32 = Color32::WHITE;
pub const TEXT: Color32 = Color32::from_rgb(0x1F, 0x23, 0x28);
pub const MUTED: Color32 = Color32::from_rgb(0x5F, 0x66, 0x73);
pub const BORDER: Color32 = Color32::from_rgb(0xE1, 0xE3, 0xE8);
pub const DISABLED: Color32 = Color32::from_rgb(0xB4, 0xB8, 0xC0);
pub const OK: Color32 = Color32::from_rgb(0x1B, 0x7F, 0x3B);
pub const OK_SOFT: Color32 = Color32::from_rgb(0xE6, 0xF4, 0xEA);
pub const WARN: Color32 = Color32::from_rgb(0x9A, 0x5B, 0x00);
pub const WARN_SOFT: Color32 = Color32::from_rgb(0xFF, 0xF3, 0xDC);
pub const ERR: Color32 = Color32::from_rgb(0xB3, 0x26, 0x1E);
pub const ERR_SOFT: Color32 = Color32::from_rgb(0xFD, 0xEB, 0xEA);

pub const SEMIBOLD: &str = "semibold";

pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(SEMIBOLD.into()))
}

/// Memakai huruf Segoe UI bawaan Windows (jika ada) agar tampilan terasa native
/// dan semua karakter Indonesia serta simbol tampil benar.
pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let dir = std::path::Path::new(r"C:\Windows\Fonts");
    let mut load = |key: &str, file: &str| -> bool {
        match std::fs::read(dir.join(file)) {
            Ok(bytes) => {
                fonts.font_data.insert(key.to_owned(), Arc::new(FontData::from_owned(bytes)));
                true
            }
            Err(_) => false,
        }
    };
    let has_regular = load("segoe", "segoeui.ttf");
    let has_semibold = load("segoe-sb", "seguisb.ttf");
    let has_symbol = load("segoe-sym", "seguisym.ttf");

    let mut prop = Vec::new();
    if has_regular {
        prop.push("segoe".to_owned());
    }
    prop.extend(fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default());
    if has_symbol {
        prop.push("segoe-sym".to_owned());
    }
    fonts.families.insert(FontFamily::Proportional, prop.clone());

    let mut sb = Vec::new();
    if has_semibold {
        sb.push("segoe-sb".to_owned());
    }
    sb.extend(prop);
    fonts.families.insert(FontFamily::Name(SEMIBOLD.into()), sb);
    ctx.set_fonts(fonts);

    ctx.style_mut(|s| {
        s.text_styles = [
            (TextStyle::Heading, semibold(24.0)),
            (TextStyle::Body, FontId::proportional(16.0)),
            (TextStyle::Button, FontId::proportional(16.0)),
            (TextStyle::Small, FontId::proportional(13.5)),
            (TextStyle::Monospace, FontId::monospace(14.0)),
        ]
        .into();
        s.spacing.item_spacing = egui::vec2(10.0, 10.0);
        s.spacing.button_padding = egui::vec2(14.0, 8.0);
        s.spacing.interact_size.y = 34.0;
        s.spacing.icon_width = 20.0;
        s.spacing.icon_width_inner = 10.0;

        let v = &mut s.visuals;
        *v = egui::Visuals::light();
        v.panel_fill = BG;
        v.window_fill = CARD;
        v.override_text_color = Some(TEXT);
        v.selection.bg_fill = ACCENT;
        v.selection.stroke = egui::Stroke::new(1.5_f32, ACCENT);
        v.hyperlink_color = ACCENT;
        v.widgets.inactive.corner_radius = 8.0.into();
        v.widgets.hovered.corner_radius = 8.0.into();
        v.widgets.active.corner_radius = 8.0.into();
        v.widgets.inactive.weak_bg_fill = Color32::from_rgb(0xEE, 0xEF, 0xF2);
        v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0xE4, 0xE6, 0xEB);
        v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, Color32::from_rgb(0xC9, 0xCC, 0xD3));
        v.widgets.active.weak_bg_fill = Color32::from_rgb(0xD9, 0xDC, 0xE2);
        // Kotak isian (nama file, rentang halaman) harus jelas terlihat sebagai kotak.
        v.extreme_bg_color = Color32::from_rgb(0xF7, 0xF8, 0xFA);
        v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, Color32::from_rgb(0xC9, 0xCC, 0xD3));
        v.widgets.inactive.fg_stroke.color = TEXT;
        v.window_corner_radius = 14.0.into();
        v.window_shadow = egui::epaint::Shadow { offset: [0, 8], blur: 28, spread: 0, color: Color32::from_black_alpha(40) };
    });
}
