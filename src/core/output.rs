//! Menentukan nama file hasil, mencegah tabrakan nama, dan menulis secara atomik.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::AppError;

/// Nama file hasil yang belum dipakai: `nama.ext`, lalu `nama (1).ext`, `nama (2).ext`, ...
/// `reserved` berisi path yang sudah dijatah dalam pekerjaan yang sama.
pub fn unique_path(dir: &Path, stem: &str, ext: &str, reserved: &[PathBuf]) -> PathBuf {
    let taken = |p: &Path| p.exists() || reserved.iter().any(|r| r.as_path() == p);
    let mut p = dir.join(format!("{stem}.{ext}"));
    let mut n = 1;
    while taken(&p) {
        p = dir.join(format!("{stem} ({n}).{ext}"));
        n += 1;
    }
    p
}

/// Membersihkan nama file dari karakter yang tidak boleh dipakai di Windows.
pub fn sanitize_stem(s: &str, fallback: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| if "<>:\"/\\|?*".contains(c) || c.is_control() { '_' } else { c })
        .collect();
    let mut cleaned = cleaned.trim().to_string();
    for ext in [".pdf", ".docx"] {
        if cleaned.to_ascii_lowercase().ends_with(ext) {
            cleaned.truncate(cleaned.len() - ext.len());
            break;
        }
    }
    let cleaned = cleaned.trim().trim_end_matches('.').trim().to_string();
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned
    }
}

/// Nama file tanpa ekstensi.
pub fn stem_of(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "hasil".to_string())
}

/// Path sementara di folder yang sama, dipakai sebelum file hasil selesai ditulis.
pub fn temp_for(final_path: &Path) -> PathBuf {
    let name = final_path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    final_path.with_file_name(format!("~{name}.part"))
}

/// Mengganti nama file sementara menjadi nama akhir.
pub fn commit(tmp: &Path, final_path: &Path) -> Result<(), AppError> {
    fs::rename(tmp, final_path).map_err(|e| {
        let _ = fs::remove_file(tmp);
        AppError::Io(e)
    })
}

/// Memastikan folder ada dan bisa ditulisi, dengan membuat lalu menghapus file uji.
pub fn check_writable(dir: &Path) -> Result<(), AppError> {
    let invalid = || AppError::OutputDirInvalid(dir.to_path_buf());
    if !dir.is_dir() {
        return Err(invalid());
    }
    let probe = dir.join(".pdfcatcher-uji-tulis.tmp");
    fs::write(&probe, b"ok").map_err(|_| invalid())?;
    let _ = fs::remove_file(&probe);
    Ok(())
}

/// Menghapus file-file yang sudah terlanjur dibuat ketika pekerjaan gagal atau dibatalkan.
pub fn cleanup(paths: &[PathBuf]) {
    for p in paths {
        let _ = fs::remove_file(p);
        let _ = fs::remove_file(temp_for(p));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nama_unik() {
        let dir = std::env::temp_dir().join(format!("pdfcatcher-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("a.pdf"), b"x").unwrap();
        fs::write(dir.join("a (1).pdf"), b"x").unwrap();
        assert_eq!(unique_path(&dir, "a", "pdf", &[]), dir.join("a (2).pdf"));
        assert_eq!(unique_path(&dir, "b", "pdf", &[dir.join("b.pdf")]), dir.join("b (1).pdf"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn bersihkan_nama() {
        assert_eq!(sanitize_stem("lap:oran/2026?", "x"), "lap_oran_2026_");
        assert_eq!(sanitize_stem("  gabungan.PDF ", "x"), "gabungan");
        assert_eq!(sanitize_stem("   ", "hasil"), "hasil");
        assert_eq!(sanitize_stem("surat.docx", "x"), "surat");
    }
}
