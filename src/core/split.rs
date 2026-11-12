use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};

use pdfium_render::prelude::*;

use super::{open_pdf, output, pdf_err, Reporter};
use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SplitMode {
    EveryPage,
    EveryN(usize),
    /// Nomor halaman mulai dari 1.
    Ranges(Vec<RangeInclusive<usize>>),
}

/// Mengubah mode pisah menjadi daftar rentang halaman.
pub fn plan(mode: &SplitMode, page_count: usize) -> Vec<RangeInclusive<usize>> {
    match mode {
        SplitMode::EveryPage => (1..=page_count).map(|p| p..=p).collect(),
        SplitMode::EveryN(n) => {
            let n = (*n).max(1);
            (0..page_count.div_ceil(n))
                .map(|k| (k * n + 1)..=((k + 1) * n).min(page_count))
                .collect()
        }
        SplitMode::Ranges(r) => r.clone(),
    }
}

/// Nama file untuk satu bagian: `prefix_p3.pdf` atau `prefix_p1-3.pdf`.
pub fn part_stem(prefix: &str, r: &RangeInclusive<usize>) -> String {
    if r.start() == r.end() {
        format!("{prefix}_p{}", r.start())
    } else {
        format!("{prefix}_p{}-{}", r.start(), r.end())
    }
}

/// Memecah satu PDF menjadi beberapa file di `out_dir`.
/// Jika gagal atau dibatalkan, semua file yang sudah dibuat dalam pekerjaan ini dihapus.
pub fn split(
    pdfium: &Pdfium,
    input: &Path,
    mode: &SplitMode,
    out_dir: &Path,
    prefix: &str,
    rep: &mut Reporter,
) -> Result<Vec<PathBuf>, AppError> {
    let src = open_pdf(pdfium, input)?;
    let page_count = src.pages().len() as usize;
    let ranges = plan(mode, page_count);
    if ranges.iter().any(|r| *r.start() == 0 || *r.end() > page_count || r.start() > r.end()) {
        return Err(AppError::BadRange(format!("melebihi {page_count} halaman")));
    }

    let mut done: Vec<PathBuf> = Vec::new();
    let result = (|| {
        for (i, r) in ranges.iter().enumerate() {
            let target = output::unique_path(out_dir, &part_stem(prefix, r), "pdf", &done);
            let mut part = pdfium.create_new_pdf().map_err(pdf_err)?;
            let a = (*r.start() - 1) as PdfPageIndex;
            let b = (*r.end() - 1) as PdfPageIndex;
            part.pages_mut()
                .copy_page_range_from_document(&src, a..=b, 0)
                .map_err(pdf_err)?;
            let tmp = output::temp_for(&target);
            part.save_to_file(&tmp).map_err(|e| {
                let _ = std::fs::remove_file(&tmp);
                pdf_err(e)
            })?;
            drop(part);
            output::commit(&tmp, &target)?;
            done.push(target);
            rep.step(i + 1, ranges.len())?;
        }
        Ok(())
    })();

    match result {
        Ok(()) => Ok(done),
        Err(e) => {
            output::cleanup(&done);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rencana_tiap_n() {
        assert_eq!(plan(&SplitMode::EveryN(3), 7), vec![1..=3, 4..=6, 7..=7]);
        assert_eq!(plan(&SplitMode::EveryN(5), 5), vec![1..=5]);
        assert_eq!(plan(&SplitMode::EveryPage, 3), vec![1..=1, 2..=2, 3..=3]);
    }

    #[test]
    fn nama_bagian() {
        assert_eq!(part_stem("laporan", &(4..=4)), "laporan_p4");
        assert_eq!(part_stem("laporan", &(1..=3)), "laporan_p1-3");
    }
}
