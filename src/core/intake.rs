//! "Menangkap" file: menyaring PDF dari file/folder yang dilepas, lalu memeriksanya.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use pdfium_render::prelude::*;

use crate::error::AppError;

/// Hasil penyaringan file yang dilepas pengguna.
#[derive(Debug, Default)]
pub struct Collected {
    pub pdfs: Vec<PathBuf>,
    /// Jumlah file yang dilepas langsung tetapi bukan PDF.
    pub skipped: usize,
}

/// Menyaring daftar path: file PDF diambil, folder ditelusuri satu tingkat (tanpa subfolder).
pub fn collect(paths: &[PathBuf]) -> Collected {
    let mut c = Collected::default();
    for p in paths {
        if p.is_dir() {
            let mut inside: Vec<PathBuf> = fs::read_dir(p)
                .map(|rd| rd.flatten().map(|e| e.path()).filter(|q| q.is_file() && is_pdf(q)).collect())
                .unwrap_or_default();
            inside.sort_by_key(|q| q.file_name().map(|n| n.to_string_lossy().to_lowercase()));
            c.pdfs.extend(inside);
        } else if p.is_file() && is_pdf(p) {
            c.pdfs.push(p.clone());
        } else {
            c.skipped += 1;
        }
    }
    c
}

/// PDF dikenali dari ekstensi `.pdf` atau dari penanda `%PDF-` di awal isi file.
pub fn is_pdf(p: &Path) -> bool {
    let ext_ok = p
        .extension()
        .map(|e| e.to_string_lossy().eq_ignore_ascii_case("pdf"))
        .unwrap_or(false);
    ext_ok || has_pdf_header(p)
}

fn has_pdf_header(p: &Path) -> bool {
    let mut buf = [0u8; 1024];
    let Ok(mut f) = fs::File::open(p) else { return false };
    let n = f.read(&mut buf).unwrap_or(0);
    buf[..n].windows(5).any(|w| w == b"%PDF-")
}

/// Kunci pembanding untuk mencegah file yang sama masuk dua kali.
pub fn same_file_key(p: &Path) -> String {
    let abs = fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    abs.to_string_lossy().to_lowercase()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Ready,
    Encrypted,
    Broken,
}

/// Informasi satu file di keranjang tangkapan.
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub pages: usize,
    pub status: FileStatus,
    /// `false` jika beberapa halaman pertama tidak punya lapisan teks (kemungkinan hasil scan).
    pub has_text: bool,
}

/// Membuka file dengan Pdfium untuk membaca jumlah halaman dan status.
pub fn inspect(pdfium: &Pdfium, path: &Path) -> FileInfo {
    let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let mut info = FileInfo {
        path: path.to_path_buf(),
        size,
        pages: 0,
        status: FileStatus::Broken,
        has_text: false,
    };
    match super::open_pdf(pdfium, path) {
        Ok(doc) => {
            info.status = FileStatus::Ready;
            info.pages = doc.pages().len() as usize;
            info.has_text = doc.pages().iter().take(5).any(|page| {
                page.text().map(|t| !t.all().trim().is_empty()).unwrap_or(false)
            });
        }
        Err(AppError::Encrypted(_)) => info.status = FileStatus::Encrypted,
        Err(_) => info.status = FileStatus::Broken,
    }
    info
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saring_file_dan_folder() {
        let dir = std::env::temp_dir().join(format!("pdfcatcher-intake-{}", std::process::id()));
        let sub = dir.join("sub");
        fs::create_dir_all(&sub).unwrap();
        fs::write(dir.join("b.pdf"), b"%PDF-1.7").unwrap();
        fs::write(dir.join("a.PDF"), b"%PDF-1.4").unwrap();
        fs::write(dir.join("salah-ekstensi.bin"), b"\n%PDF-1.5").unwrap();
        fs::write(dir.join("catatan.txt"), b"halo").unwrap();
        fs::write(sub.join("dalam.pdf"), b"%PDF-1.7").unwrap();

        let c = collect(&[dir.clone(), dir.join("catatan.txt")]);
        let names: Vec<String> = c
            .pdfs
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["a.PDF", "b.pdf", "salah-ekstensi.bin"]);
        assert_eq!(c.skipped, 1);
        fs::remove_dir_all(&dir).unwrap();
    }
}
