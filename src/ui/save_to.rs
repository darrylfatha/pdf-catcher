//! Baris "Simpan ke": folder tujuan + nama file hasil (kecuali aksi batch, yang memakai
//! nama file sumbernya masing-masing).

use egui::RichText;

use super::theme::*;
use super::widgets;
use crate::app::{Action, App};
use pdf_catcher::settings::OutputMode;

pub fn row(app: &mut App, ui: &mut egui::Ui, a: Action) {
    if a.is_batch() {
        ui.label(
            RichText::new("Setiap file akan menghasilkan satu PDF dengan nama yang sama (otomatis ditambah (1), (2) jika sudah ada).")
                .color(MUTED)
                .size(13.5),
        );
        ui.add_space(6.0);
    } else {
        let default = app.default_name(a);
        let (label, ext) = match a {
            Action::Merge | Action::ImagesToPdf => ("Nama file hasil:", ".pdf"),
            Action::Split => ("Awalan nama file:", "_p1.pdf, …"),
            Action::Convert => ("Nama file hasil:", ".docx"),
            Action::WordToPdf | Action::ExcelToPdf | Action::PptToPdf => unreachable!("aksi batch ditangani di atas"),
        };
        ui.horizontal(|ui| {
            ui.label(RichText::new(label).size(15.5));
            if let Some(field) = app.name_field(a) {
                ui.add(egui::TextEdit::singleline(field).hint_text(default).desired_width(260.0));
            }
            ui.label(RichText::new(ext).color(MUTED));
        });
        ui.label(
            RichText::new("Jika nama sudah dipakai, otomatis ditambah (1), (2), dan seterusnya. File asli tidak pernah ditimpa.")
                .color(MUTED)
                .size(13.0),
        );
        ui.add_space(6.0);
    }

    // Folder tujuan
    let target = app.target_dir();
    ui.horizontal(|ui| {
        ui.label(RichText::new("Simpan ke:").size(15.5));
        let text = match &target {
            Some(d) => d.display().to_string(),
            None => "Akan ditanyakan saat menyimpan".to_string(),
        };
        egui::Frame::default()
            .fill(egui::Color32::from_rgb(0xF4, 0xF5, 0xF7))
            .corner_radius(8.0)
            .inner_margin(egui::Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.set_max_width((ui.available_width() - 170.0).max(200.0));
                ui.add(egui::Label::new(RichText::new(&text).size(15.0)).truncate())
                    .on_hover_text(&text);
            });
        if widgets::secondary_button(ui, "Ganti folder…")
            .on_hover_text("Pilih folder lain khusus untuk pekerjaan ini")
            .clicked()
        {
            let mut dlg = rfd::FileDialog::new().set_title("Pilih folder untuk menyimpan hasil");
            if let Some(d) = target.clone().or_else(|| app.source_dir()) {
                dlg = dlg.set_directory(d);
            }
            if let Some(d) = dlg.pick_folder() {
                app.dir_override = Some(d);
            }
        }
    });

    let source_note = match (app.dir_override.is_some(), app.settings.output_mode) {
        (true, _) => None,
        (false, OutputMode::SameAsSource) => Some("Sama dengan folder file asli (bisa diubah di Pengaturan)."),
        (false, OutputMode::Custom) => Some("Folder default dari Pengaturan."),
        (false, OutputMode::Ask) => Some("Pengaturan: tanya setiap kali menyimpan."),
    };
    if let Some(note) = source_note {
        ui.label(RichText::new(note).color(MUTED).size(13.0));
    }
    if app.dir_override.is_some() {
        ui.horizontal(|ui| {
            ui.checkbox(&mut app.make_default, RichText::new("Jadikan folder default untuk seterusnya").size(15.0));
            if ui.link("Kembalikan ke folder semula").clicked() {
                app.dir_override = None;
                app.make_default = false;
            }
        });
    }
}
