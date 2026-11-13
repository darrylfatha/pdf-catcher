//! Logika inti PDF Catcher. Tidak tahu apa-apa soal UI.

pub mod convert;
pub mod images;
pub mod intake;
pub mod layout;
pub mod merge;
pub mod office;
pub mod output;
pub mod ranges;
pub mod split;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use pdfium_render::prelude::*;

use crate::error::AppError;

/// Folder tempat `pdfcatcher.exe` berada.
pub fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Memuat `pdfium.dll` dari folder yang diberikan.
pub fn load_pdfium(dir: &Path) -> Result<Pdfium, AppError> {
    let lib = Pdfium::pdfium_platform_library_name_at_path(dir);
    if !lib.exists() {
        return Err(AppError::PdfiumMissing);
    }
    let bindings = Pdfium::bind_to_library(lib).map_err(|_| AppError::PdfiumMissing)?;
    Ok(Pdfium::new(bindings))
}

/// Membuka PDF dan menerjemahkan error Pdfium menjadi `AppError`.
pub fn open_pdf<'a>(pdfium: &'a Pdfium, path: &Path) -> Result<PdfDocument<'a>, AppError> {
    pdfium.load_pdf_from_file(path, None).map_err(|e| match e {
        PdfiumError::PdfiumLibraryInternalError(
            PdfiumInternalError::PasswordError | PdfiumInternalError::SecurityError,
        ) => AppError::Encrypted(path.to_path_buf()),
        PdfiumError::IoError(io) => AppError::Io(io),
        _ => AppError::NotPdf(path.to_path_buf()),
    })
}

pub(crate) fn pdf_err(e: PdfiumError) -> AppError {
    AppError::Pdf(format!("{e:?}"))
}

/// Penghubung antara fungsi inti dan pemanggilnya: laporan progres + bendera batal.
pub struct Reporter<'a> {
    cancel: &'a AtomicBool,
    progress: &'a mut dyn FnMut(usize, usize),
}

impl<'a> Reporter<'a> {
    pub fn new(cancel: &'a AtomicBool, progress: &'a mut dyn FnMut(usize, usize)) -> Self {
        Self { cancel, progress }
    }

    /// Melaporkan progres lalu memeriksa apakah pengguna menekan Batal.
    pub fn step(&mut self, done: usize, total: usize) -> Result<(), AppError> {
        (self.progress)(done, total);
        self.check()
    }

    pub fn check(&self) -> Result<(), AppError> {
        if self.cancel.load(Ordering::Relaxed) {
            Err(AppError::Cancelled)
        } else {
            Ok(())
        }
    }
}
