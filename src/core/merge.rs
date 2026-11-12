use std::path::{Path, PathBuf};

use pdfium_render::prelude::*;

use super::{open_pdf, output, pdf_err, Reporter};
use crate::error::AppError;

/// Menggabungkan beberapa PDF (sesuai urutan) menjadi satu file di `output`.
/// `output` harus belum ada; file ditulis ke path sementara dulu lalu diganti namanya.
pub fn merge(
    pdfium: &Pdfium,
    inputs: &[PathBuf],
    output: &Path,
    rep: &mut Reporter,
) -> Result<PathBuf, AppError> {
    let total = inputs.len();
    let mut dest = pdfium.create_new_pdf().map_err(pdf_err)?;
    for (i, path) in inputs.iter().enumerate() {
        let src = open_pdf(pdfium, path)?;
        dest.pages_mut().append(&src).map_err(pdf_err)?;
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
