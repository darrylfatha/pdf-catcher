//! Pengaturan yang diingat antar sesi, disimpan di `config.json`.
//!
//! Lokasi utama: folder instalasi (di samping `pdfcatcher.exe`) agar tidak menambah data
//! di drive C. Jika folder itu tidak bisa ditulisi, dipakai `%APPDATA%\PDF Catcher\config.json`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    /// Simpan di folder yang sama dengan file sumber.
    #[default]
    SameAsSource,
    /// Simpan di satu folder pilihan pengguna.
    Custom,
    /// Tanyakan setiap kali aksi dijalankan.
    Ask,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub output_mode: OutputMode,
    #[serde(default)]
    pub custom_dir: Option<PathBuf>,
}

const FILE_NAME: &str = "config.json";

/// Menentukan lokasi `config.json`.
pub fn config_path() -> PathBuf {
    let beside_exe = crate::core::exe_dir().join(FILE_NAME);
    if beside_exe.exists() || dir_writable(&crate::core::exe_dir()) {
        return beside_exe;
    }
    let appdata = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    appdata.join("PDF Catcher").join(FILE_NAME)
}

fn dir_writable(dir: &Path) -> bool {
    let probe = dir.join(".pdfcatcher-uji-tulis.tmp");
    let ok = fs::write(&probe, b"ok").is_ok();
    let _ = fs::remove_file(&probe);
    ok
}

impl Settings {
    pub fn load(path: &Path) -> Settings {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        fs::write(path, json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simpan_dan_muat() {
        let p = std::env::temp_dir().join(format!("pdfcatcher-cfg-{}.json", std::process::id()));
        let s = Settings { output_mode: OutputMode::Custom, custom_dir: Some(PathBuf::from(r"D:\Hasil")) };
        s.save(&p).unwrap();
        let text = fs::read_to_string(&p).unwrap();
        assert!(text.contains("\"custom\""));
        let back = Settings::load(&p);
        assert_eq!(back.output_mode, OutputMode::Custom);
        assert_eq!(back.custom_dir, Some(PathBuf::from(r"D:\Hasil")));
        fs::remove_file(&p).unwrap();
    }

    #[test]
    fn default_folder_sumber() {
        let s: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(s.output_mode, OutputMode::SameAsSource);
    }
}
