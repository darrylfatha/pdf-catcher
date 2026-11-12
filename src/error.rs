use std::path::{Path, PathBuf};

/// Semua kesalahan yang bisa muncul, dengan pesan yang ramah pengguna.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("File pdfium.dll tidak ditemukan di folder aplikasi. Pasang ulang PDF Catcher.")]
    PdfiumMissing,

    #[error("File \"{}\" tidak dapat dibaca sebagai PDF.", file_name(.0))]
    NotPdf(PathBuf),

    #[error("PDF berpassword belum didukung: \"{}\".", file_name(.0))]
    Encrypted(PathBuf),

    #[error("Rentang halaman tidak valid: \"{0}\". Contoh: 1-3, 5, 8-10")]
    BadRange(String),

    #[error("PDF \"{}\" berupa gambar (hasil scan), teksnya tidak dapat diambil.", file_name(.0))]
    NoText(PathBuf),

    #[error("Folder tujuan tidak dapat dipakai, pilih folder lain: {}", .0.display())]
    OutputDirInvalid(PathBuf),

    #[error("Gagal menyimpan file. Tutup file tujuan jika sedang dibuka, lalu coba lagi. ({0})")]
    Io(#[from] std::io::Error),

    #[error("Pdfium gagal memproses file: {0}")]
    Pdf(String),

    #[error("Proses dibatalkan.")]
    Cancelled,
}

pub fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| p.display().to_string())
}
