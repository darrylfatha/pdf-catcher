//! Membuat PDF contoh di tests/fixtures untuk uji manual:
//!   cargo run --example make_fixtures

use std::path::PathBuf;

use pdfium_render::prelude::*;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let pdfium = pdf_catcher::core::load_pdfium(&root.join("vendor")).expect("vendor/pdfium.dll");
    let out = root.join("tests").join("fixtures");
    std::fs::create_dir_all(&out).unwrap();

    let make = |name: &str, pages: usize, text: bool| {
        let mut doc = pdfium.create_new_pdf().unwrap();
        let font = doc.fonts_mut().helvetica();
        let bold = doc.fonts_mut().helvetica_bold();
        for i in 1..=pages {
            let mut page = doc.pages_mut().create_page_at_end(PdfPagePaperSize::a4()).unwrap();
            if text {
                let objs = page.objects_mut();
                objs.create_text_object(PdfPoints::new(72.0), PdfPoints::new(760.0), format!("{name} - Halaman {i}"), bold, PdfPoints::new(22.0)).unwrap();
                let lines = [
                    "PDF Catcher menangkap file PDF lalu mengolahnya sesuai pilihan.",
                    "Baris ini adalah lanjutan dari paragraf pertama pada halaman ini.",
                ];
                for (k, l) in lines.iter().enumerate() {
                    objs.create_text_object(PdfPoints::new(72.0), PdfPoints::new(700.0 - 15.0 * k as f32), *l, font, PdfPoints::new(12.0)).unwrap();
                }
                objs.create_text_object(PdfPoints::new(72.0), PdfPoints::new(640.0), "Paragraf kedua dimulai di sini.", font, PdfPoints::new(12.0)).unwrap();
            }
        }
        let path = out.join(format!("{name}.pdf"));
        doc.save_to_file(&path).unwrap();
        println!("{}", path.display());
    };

    make("satu-halaman", 1, true);
    make("sepuluh-halaman", 10, true);
    make("laporan-bulanan", 4, true);
    make("hasil-scan", 2, false);
}
