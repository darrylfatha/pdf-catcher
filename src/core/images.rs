//! JPG/PNG ke PDF: tiap gambar jadi satu halaman, digabung menjadi satu file PDF.

use std::path::{Path, PathBuf};

use pdfium_render::prelude::*;

use super::{output, pdf_err, Reporter};
use crate::error::AppError;

/// 1 piksel gambar dianggap 1/96 inci (96 DPI, resolusi layar umum) = 0,75 point;
/// ini membuat ukuran halaman kira-kira sama dengan ukuran gambar saat dilihat di layar.
const POINTS_PER_PIXEL: f32 = 72.0 / 96.0;

/// Pengaman untuk gambar beresolusi sangat tinggi: halaman PDF tidak dibuat lebih besar
/// dari ini (≈ 2,8 meter), supaya tetap bisa dibuka wajar oleh pembaca PDF lain.
const MAX_PAGE_POINTS: f32 = 200.0 * 72.0;

/// Menggabungkan beberapa gambar menjadi satu PDF, satu gambar per halaman, sesuai urutan.
pub fn images_to_pdf(pdfium: &Pdfium, inputs: &[PathBuf], output: &Path, rep: &mut Reporter) -> Result<PathBuf, AppError> {
    let mut dest = pdfium.create_new_pdf().map_err(pdf_err)?;
    let total = inputs.len();
    for (i, path) in inputs.iter().enumerate() {
        let img = image::open(path).map_err(|e| AppError::NotImage(path.to_path_buf(), e.to_string()))?;
        let w_pt = (img.width() as f32 * POINTS_PER_PIXEL).clamp(1.0, MAX_PAGE_POINTS);
        let h_pt = (img.height() as f32 * POINTS_PER_PIXEL).clamp(1.0, MAX_PAGE_POINTS);
        let size = PdfPagePaperSize::Custom(PdfPoints::new(w_pt), PdfPoints::new(h_pt));
        let mut page = dest.pages_mut().create_page_at_end(size).map_err(pdf_err)?;
        let obj = PdfPageImageObject::new_with_size(&dest, &img, PdfPoints::new(w_pt), PdfPoints::new(h_pt)).map_err(pdf_err)?;
        page.objects_mut().add_image_object(obj).map_err(pdf_err)?;
        rep.step(i + 1, total)?;
    }
    let tmp = output::temp_for(output);
    dest.save_to_file(&tmp).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        pdf_err(e)
    })?;
    drop(dest);
    if let Err(e) = rep.check() {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    output::commit(&tmp, output)?;
    Ok(output.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn gambar_ke_pdf() {
        let vendor = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor");
        let pdfium = super::super::load_pdfium(&vendor).expect("vendor/pdfium.dll harus ada");

        let dir = std::env::temp_dir().join(format!("pdfcatcher-img-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.png");
        let b = dir.join("b.png");
        image::RgbImage::new(200, 100).save(&a).unwrap();
        image::RgbImage::new(50, 300).save(&b).unwrap();

        let cancel = AtomicBool::new(false);
        let mut cb = |_, _| {};
        let mut rep = Reporter::new(&cancel, &mut cb);
        let target = dir.join("gabungan.pdf");
        let out = images_to_pdf(&pdfium, &[a, b], &target, &mut rep).unwrap();

        let doc = pdfium.load_pdf_from_file(&out, None).unwrap();
        assert_eq!(doc.pages().len(), 2);
        let p0 = doc.pages().get(0).unwrap();
        let p1 = doc.pages().get(1).unwrap();
        assert!((p0.width().value - 200.0 * POINTS_PER_PIXEL).abs() < 0.5);
        assert!((p0.height().value - 100.0 * POINTS_PER_PIXEL).abs() < 0.5);
        assert!((p1.width().value - 50.0 * POINTS_PER_PIXEL).abs() < 0.5);
        assert!((p1.height().value - 300.0 * POINTS_PER_PIXEL).abs() < 0.5);

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
