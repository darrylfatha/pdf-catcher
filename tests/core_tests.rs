//! Test inti dengan Pdfium sungguhan (vendor/pdfium.dll).
//! Semua skenario dijalankan dalam satu test karena Pdfium hanya boleh dimuat sekali per proses.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use pdf_catcher::core::split::SplitMode;
use pdf_catcher::core::{convert, intake, load_pdfium, merge, output, split, Reporter};
use pdf_catcher::error::AppError;
use pdfium_render::prelude::*;

fn make_pdf(pdfium: &Pdfium, path: &Path, pages: usize, with_text: bool) {
    let mut doc = pdfium.create_new_pdf().unwrap();
    let font = doc.fonts_mut().helvetica();
    let bold = doc.fonts_mut().helvetica_bold();
    for i in 1..=pages {
        let mut page = doc.pages_mut().create_page_at_end(PdfPagePaperSize::a4()).unwrap();
        if with_text {
            page.objects_mut()
                .create_text_object(PdfPoints::new(72.0), PdfPoints::new(760.0), format!("Judul Halaman {i}"), bold, PdfPoints::new(22.0))
                .unwrap();
            page.objects_mut()
                .create_text_object(PdfPoints::new(72.0), PdfPoints::new(700.0), "Ini paragraf pertama yang", font, PdfPoints::new(12.0))
                .unwrap();
            page.objects_mut()
                .create_text_object(PdfPoints::new(72.0), PdfPoints::new(686.0), "berlanjut ke baris kedua.", font, PdfPoints::new(12.0))
                .unwrap();
        }
    }
    doc.save_to_file(path).unwrap();
}

fn page_count(pdfium: &Pdfium, path: &Path) -> usize {
    pdfium.load_pdf_from_file(path, None).unwrap().pages().len() as usize
}

fn no_temp_files(dir: &Path) -> bool {
    std::fs::read_dir(dir).unwrap().flatten().all(|e| !e.file_name().to_string_lossy().ends_with(".part"))
}

#[test]
fn skenario_pdfium() {
    let vendor = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor");
    let pdfium = load_pdfium(&vendor).expect("vendor/pdfium.dll harus ada");

    let dir = std::env::temp_dir().join(format!("pdfcatcher-core-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let a = dir.join("satu halaman.pdf");
    let b = dir.join("sepuluh-halamán.pdf"); // nama non-ASCII
    let scan = dir.join("scan.pdf");
    make_pdf(&pdfium, &a, 1, true);
    make_pdf(&pdfium, &b, 10, true);
    make_pdf(&pdfium, &scan, 2, false);

    let cancel = AtomicBool::new(false);
    let last = std::cell::Cell::new((0usize, 0usize));
    let mut cb = |d, t| last.set((d, t));

    // Pemeriksaan file
    let info = intake::inspect(&pdfium, &b);
    assert_eq!(info.status, intake::FileStatus::Ready);
    assert_eq!(info.pages, 10);
    assert!(info.has_text);
    assert!(!intake::inspect(&pdfium, &scan).has_text);
    let junk = dir.join("rusak.pdf");
    std::fs::write(&junk, b"%PDF-1.7 bukan pdf sungguhan").unwrap();
    assert_eq!(intake::inspect(&pdfium, &junk).status, intake::FileStatus::Broken);

    // Gabung: 1 + 10 + 1 halaman
    {
        let mut rep = Reporter::new(&cancel, &mut cb);
        let target = output::unique_path(&dir, "gabungan", "pdf", &[]);
        let out = merge::merge(&pdfium, &[a.clone(), b.clone(), a.clone()], &target, &mut rep).unwrap();
        assert_eq!(page_count(&pdfium, &out), 12);
    }
    assert_eq!(last.get(), (3, 3));

    // Gabung lagi dengan nama yang sama -> "gabungan (1).pdf"
    {
        let mut rep = Reporter::new(&cancel, &mut cb);
        let target = output::unique_path(&dir, "gabungan", "pdf", &[]);
        assert_eq!(target.file_name().unwrap(), "gabungan (1).pdf");
        merge::merge(&pdfium, &[a.clone(), a.clone()], &target, &mut rep).unwrap();
    }

    // Pisah: rentang kustom 1-3, 5, 8-10 -> 3 file
    let parts_dir = dir.join("pisah");
    std::fs::create_dir_all(&parts_dir).unwrap();
    {
        let mut rep = Reporter::new(&cancel, &mut cb);
        let ranges = pdf_catcher::core::ranges::parse_ranges("1-3, 5, 8-10", 10).unwrap();
        let files = split::split(&pdfium, &b, &SplitMode::Ranges(ranges), &parts_dir, "bab", &mut rep).unwrap();
        let names: Vec<String> = files.iter().map(|f| f.file_name().unwrap().to_string_lossy().into_owned()).collect();
        assert_eq!(names, vec!["bab_p1-3.pdf", "bab_p5.pdf", "bab_p8-10.pdf"]);
        let counts: Vec<usize> = files.iter().map(|f| page_count(&pdfium, f)).collect();
        assert_eq!(counts, vec![3, 1, 3]);
    }
    // Pisah tiap halaman
    {
        let mut rep = Reporter::new(&cancel, &mut cb);
        let files = split::split(&pdfium, &b, &SplitMode::EveryPage, &parts_dir, "hal", &mut rep).unwrap();
        assert_eq!(files.len(), 10);
    }
    // Pisah tiap 4 halaman -> 4, 4, 2
    {
        let mut rep = Reporter::new(&cancel, &mut cb);
        let files = split::split(&pdfium, &b, &SplitMode::EveryN(4), &parts_dir, "per4", &mut rep).unwrap();
        let counts: Vec<usize> = files.iter().map(|f| page_count(&pdfium, f)).collect();
        assert_eq!(counts, vec![4, 4, 2]);
    }

    // Batal di tengah pisah: tidak ada file tersisa
    {
        let cancel_now = AtomicBool::new(true);
        let mut cb2 = |_, _| {};
        let mut rep = Reporter::new(&cancel_now, &mut cb2);
        let cancel_dir = dir.join("batal");
        std::fs::create_dir_all(&cancel_dir).unwrap();
        let r = split::split(&pdfium, &b, &SplitMode::EveryPage, &cancel_dir, "x", &mut rep);
        assert!(matches!(r, Err(AppError::Cancelled)));
        assert_eq!(std::fs::read_dir(&cancel_dir).unwrap().count(), 0);
    }

    // PDF ke DOCX
    {
        let mut rep = Reporter::new(&cancel, &mut cb);
        let target = output::unique_path(&dir, "sepuluh", "docx", &[]);
        let out = convert::pdf_to_docx(&pdfium, &b, Some(&[2..=3]), &target, &mut rep).unwrap();
        let bytes = std::fs::read(&out).unwrap();
        assert!(bytes.len() > 1000);
        let docx = docx_rs::read_docx(&bytes).expect("DOCX bisa dibaca kembali");
        let json = docx.json();
        assert!(json.contains("Judul Halaman 2"), "judul halaman 2 harus ada");
        assert!(json.contains("Ini paragraf pertama yang berlanjut ke baris kedua."), "dua baris harus jadi satu paragraf");
        assert!(!json.contains("Judul Halaman 5"), "halaman di luar rentang tidak ikut");
    }

    // PDF tanpa teks -> NoText, tanpa file tersisa
    {
        let mut rep = Reporter::new(&cancel, &mut cb);
        let target = dir.join("scan.docx");
        let r = convert::pdf_to_docx(&pdfium, &scan, None, &target, &mut rep);
        assert!(matches!(r, Err(AppError::NoText(_))));
        assert!(!target.exists());
    }

    // File bukan PDF / rusak ditolak tanpa crash
    {
        let mut rep = Reporter::new(&cancel, &mut cb);
        let r = merge::merge(&pdfium, &[a.clone(), junk.clone()], &dir.join("gagal.pdf"), &mut rep);
        assert!(matches!(r, Err(AppError::NotPdf(_))));
        assert!(!dir.join("gagal.pdf").exists());
    }

    assert!(no_temp_files(&dir));
    assert!(no_temp_files(&parts_dir));
    drop(pdfium);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn pdfium_hilang_jelas() {
    let r = load_pdfium(Path::new(r"Z:\tidak-ada"));
    assert!(matches!(r, Err(AppError::PdfiumMissing)));
}
