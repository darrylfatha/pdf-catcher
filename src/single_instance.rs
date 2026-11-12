//! Satu jendela saja: jika PDF Catcher sudah terbuka, file dari klik kanan / ikon
//! dikirim ke jendela yang ada lewat named pipe lokal, lalu proses kedua langsung keluar.
//!
//! Mutex bernama menentukan siapa proses pertama, sehingga beberapa proses yang dibuka
//! bersamaan (memilih banyak file lalu klik kanan) tetap berakhir di satu jendela.

use std::ffi::OsStr;
use std::io::{Read, Write};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{FromRawHandle, RawHandle};
use std::path::PathBuf;
use std::time::Duration;

use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, ERROR_PIPE_CONNECTED, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::PIPE_ACCESS_INBOUND;
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE,
    PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};
use windows_sys::Win32::System::Threading::CreateMutexW;

pub enum Role {
    /// Proses pertama: buka jendela dan dengarkan kiriman dari proses lain.
    Primary,
    /// File sudah diteruskan ke jendela yang terbuka; proses ini selesai.
    Forwarded,
    /// Jendela lain ada tetapi tidak menjawab; buka jendela sendiri tanpa mendengarkan.
    Standalone,
}

fn user_tag() -> String {
    std::env::var("USERNAME")
        .unwrap_or_default()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

fn pipe_name() -> String {
    format!(r"\\.\pipe\PDFCatcher-{}", user_tag())
}

fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

/// Menentukan peran proses ini. `paths` diteruskan jika sudah ada jendela lain.
pub fn acquire(paths: &[PathBuf]) -> Role {
    let name = wide(&format!("Local\\PDFCatcher-{}", user_tag()));
    // Handle mutex sengaja tidak ditutup: dilepas Windows saat proses berakhir.
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    let already = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
    if handle.is_null() || !already {
        return Role::Primary;
    }

    let mut message = String::new();
    for p in paths {
        message.push_str(&p.to_string_lossy());
        message.push('\n');
    }
    for _ in 0..60 {
        if let Ok(mut f) = std::fs::OpenOptions::new().write(true).open(pipe_name()) {
            if f.write_all(message.as_bytes()).is_ok() {
                return Role::Forwarded;
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Role::Standalone
}

/// Menjalankan thread pendengar. `on_paths` dipanggil untuk setiap kiriman
/// (daftar kosong berarti pengguna hanya membuka aplikasi lagi).
pub fn serve(on_paths: impl Fn(Vec<PathBuf>) + Send + 'static) {
    std::thread::Builder::new()
        .name("pdfcatcher-pipe".into())
        .spawn(move || {
            let name = wide(&pipe_name());
            loop {
                let h = unsafe {
                    CreateNamedPipeW(
                        name.as_ptr(),
                        PIPE_ACCESS_INBOUND,
                        PIPE_TYPE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                        PIPE_UNLIMITED_INSTANCES,
                        0,
                        64 * 1024,
                        0,
                        std::ptr::null(),
                    )
                };
                if h == INVALID_HANDLE_VALUE {
                    return;
                }
                let ok = unsafe { ConnectNamedPipe(h, std::ptr::null_mut()) };
                if ok == 0 && unsafe { GetLastError() } != ERROR_PIPE_CONNECTED {
                    unsafe { CloseHandle(h) };
                    continue;
                }
                // File mengambil alih handle dan menutupnya saat di-drop.
                let mut f = unsafe { std::fs::File::from_raw_handle(h as RawHandle) };
                let mut buf = Vec::new();
                let _ = f.read_to_end(&mut buf);
                drop(f);
                let paths = String::from_utf8_lossy(&buf)
                    .lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .map(PathBuf::from)
                    .collect();
                on_paths(paths);
            }
        })
        .expect("thread pipe");
}
