//! "Menangkap" file: mengenali jenisnya dari file/folder yang dilepas, lalu memeriksanya.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use pdfium_render::prelude::*;

use crate::error::AppError;

/// Jenis file yang bisa ditangkap PDF Catcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Pdf,
    Word,
    Excel,
    PowerPoint,
    Image,
}

impl FileKind {
    pub fn label(self) -> &'static str {
        match self {
            FileKind::Pdf => "PDF",
            FileKind::Word => "Dokumen Word",
            FileKind::Excel => "Spreadsheet Excel",
            FileKind::PowerPoint => "Presentasi PowerPoint",
            FileKind::Image => "Gambar",
        }
    }
}

/// Hasil penyaringan file yang dilepas pengguna, dengan jenisnya masing-masing.
#[derive(Debug, Default)]
pub struct Collected {
    pub files: Vec<(PathBuf, FileKind)>,
    /// Jumlah file yang dilepas langsung tetapi tidak dikenali.
    pub skipped: usize,
}

/// Menyaring daftar path: file yang dikenali diambil, folder ditelusuri satu tingkat (tanpa subfolder).
pub fn collect(paths: &[PathBuf]) -> Collected {
    let mut c = Collected::default();
    for p in paths {
        if p.is_dir() {
            let mut inside: Vec<(PathBuf, FileKind)> = fs::read_dir(p)
                .map(|rd| {
                    rd.flatten()
                        .map(|e| e.path())
                        .filter(|q| q.is_file())
                        .filter_map(|q| detect_kind(&q).map(|k| (q, k)))
                        .collect()
                })
                .unwrap_or_default();
            inside.sort_by_key(|(q, _)| q.file_name().map(|n| n.to_string_lossy().to_lowercase()));
            c.files.extend(inside);
        } else if p.is_file() {
            match detect_kind(p) {
                Some(k) => c.files.push((p.clone(), k)),
                None => c.skipped += 1,
            }
        } else {
            c.skipped += 1;
        }
    }
    c
}

/// Mengenali jenis file dari ekstensi, dibantu penanda isi file untuk PDF dan gambar
/// (sehingga file dengan ekstensi salah tetap tertangkap kalau isinya memang PDF/gambar).
pub fn detect_kind(p: &Path) -> Option<FileKind> {
    if is_pdf(p) {
        return Some(FileKind::Pdf);
    }
    let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    match ext.as_str() {
        "doc" | "docx" => Some(FileKind::Word),
        "xls" | "xlsx" => Some(FileKind::Excel),
        "ppt" | "pptx" => Some(FileKind::PowerPoint),
        "jpg" | "jpeg" | "png" => Some(FileKind::Image),
        _ if has_header(p, &[0xFF, 0xD8, 0xFF]) || has_header(p, b"\x89PNG") => Some(FileKind::Image),
        _ => None,
    }
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

fn has_header(p: &Path, magic: &[u8]) -> bool {
    let mut buf = vec![0u8; magic.len()];
    let Ok(mut f) = fs::File::open(p) else { return false };
    f.read_exact(&mut buf).is_ok() && buf == magic
}

/// Untuk file berbasis zip (docx/xlsx/pptx): memastikan isinya memang arsip zip,
/// bukan file lain yang kebetulan diberi ekstensi itu.
fn looks_like_zip(p: &Path) -> bool {
    has_header(p, b"PK\x03\x04")
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

/// Informasi satu file di keranjang tangkapan. Field `pages`/`has_text` hanya berarti untuk
/// PDF; `dims` hanya untuk gambar.
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub kind: FileKind,
    pub size: u64,
    pub pages: usize,
    pub has_text: bool,
    pub dims: Option<(u32, u32)>,
    pub status: FileStatus,
}

/// Memeriksa satu file sesuai jenisnya. PDF dibuka dengan Pdfium; jenis lain diperiksa ringan
/// (ukuran, dan penanda isi file untuk format berbasis zip dan gambar) tanpa Pdfium.
pub fn inspect(pdfium: Option<&Pdfium>, path: &Path, kind: FileKind) -> FileInfo {
    let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let mut info = FileInfo {
        path: path.to_path_buf(),
        kind,
        size,
        pages: 0,
        has_text: false,
        dims: None,
        status: FileStatus::Broken,
    };
    match kind {
        FileKind::Pdf => {
            let Some(pdfium) = pdfium else { return info };
            match super::open_pdf(pdfium, path) {
                Ok(doc) => {
                    info.status = FileStatus::Ready;
                    info.pages = doc.pages().len() as usize;
                    info.has_text = doc
                        .pages()
                        .iter()
                        .take(5)
                        .any(|page| page.text().map(|t| !t.all().trim().is_empty()).unwrap_or(false));
                }
                Err(AppError::Encrypted(_)) => info.status = FileStatus::Encrypted,
                Err(_) => info.status = FileStatus::Broken,
            }
        }
        FileKind::Word | FileKind::Excel | FileKind::PowerPoint => {
            let ext_is_legacy = path
                .extension()
                .map(|e| matches!(e.to_string_lossy().to_lowercase().as_str(), "doc" | "xls" | "ppt"))
                .unwrap_or(false);
            info.status = if size == 0 {
                FileStatus::Broken
            } else if ext_is_legacy || looks_like_zip(path) {
                FileStatus::Ready
            } else {
                FileStatus::Broken
            };
        }
        FileKind::Image => match image::image_dimensions(path) {
            Ok(dims) => {
                info.status = FileStatus::Ready;
                info.dims = Some(dims);
            }
            Err(_) => info.status = FileStatus::Broken,
        },
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
            .files
            .iter()
            .map(|(p, _)| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["a.PDF", "b.pdf", "salah-ekstensi.bin"]);
        assert!(c.files.iter().all(|(_, k)| *k == FileKind::Pdf));
        assert_eq!(c.skipped, 1);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn mengenali_jenis_lain() {
        let dir = std::env::temp_dir().join(format!("pdfcatcher-kind-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("surat.docx"), b"PK\x03\x04bukan docx sungguhan").unwrap();
        fs::write(dir.join("rencana.xlsx"), b"PK\x03\x04bukan xlsx sungguhan").unwrap();
        fs::write(dir.join("presentasi.pptx"), b"PK\x03\x04bukan pptx sungguhan").unwrap();
        fs::write(dir.join("foto.jpg"), [0xFFu8, 0xD8, 0xFF, 0xE0]).unwrap();
        fs::write(dir.join("logo.png"), b"\x89PNG\r\n\x1a\n").unwrap();
        fs::write(dir.join("rusak.docx"), b"bukan zip sama sekali").unwrap();

        assert_eq!(detect_kind(&dir.join("surat.docx")), Some(FileKind::Word));
        assert_eq!(detect_kind(&dir.join("rencana.xlsx")), Some(FileKind::Excel));
        assert_eq!(detect_kind(&dir.join("presentasi.pptx")), Some(FileKind::PowerPoint));
        assert_eq!(detect_kind(&dir.join("foto.jpg")), Some(FileKind::Image));
        assert_eq!(detect_kind(&dir.join("logo.png")), Some(FileKind::Image));

        assert_eq!(inspect(None, &dir.join("surat.docx"), FileKind::Word).status, FileStatus::Ready);
        assert_eq!(inspect(None, &dir.join("rusak.docx"), FileKind::Word).status, FileStatus::Broken);

        fs::remove_dir_all(&dir).unwrap();
    }
}
