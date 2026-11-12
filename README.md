# PDF Catcher

Aplikasi desktop Windows untuk **menangkap** file PDF lalu mengolahnya: gabung, pisah, atau ubah ke Word.
Semua proses berjalan di komputer sendiri, tanpa internet dan tanpa unggah file.

Dibuat dengan Rust + egui (UI native) dan Pdfium (mesin PDF milik Chrome).

## Cara pakai

1. **Tangkap PDF.** Seret file PDF (atau satu folder) ke jendela, klik kotak tangkap, atau klik kanan file PDF lalu pilih *Buka dengan PDF Catcher*.
2. **Pilih aksi.** Gabung (2 PDF atau lebih), Pisah (1 PDF), atau Ke Word (1 PDF).
3. **Atur & simpan.** Hasil disimpan di folder file asli (default), atau di folder lain lewat *Ganti folder…*. Nama yang sudah dipakai otomatis diberi akhiran (1), (2); file asli tidak pernah ditimpa.

Folder hasil default bisa diubah di **Pengaturan**: sama dengan folder file asli, satu folder pilihan, atau tanya setiap kali.

## Struktur

```
src/
  main.rs, app.rs        jendela, state, alur tangkap -> aksi -> simpan
  ui/                    tampilan (zona tangkap, keranjang, kartu aksi, simpan ke, pengaturan)
  lib.rs                 logika tanpa UI (bisa diuji)
  core/                  intake, merge, split, ranges, output, convert, layout
  worker.rs              satu thread pekerja yang memegang Pdfium
  settings.rs            config.json
  single_instance.rs     satu jendela (named pipe)
installer/pdfcatcher.iss skrip Inno Setup
vendor/pdfium.dll        dari github.com/bblanchon/pdfium-binaries (win-x64)
tests/                   test dengan Pdfium sungguhan + PDF contoh
```

## Build dari sumber

Butuh Rust (toolchain `x86_64-pc-windows-gnu`) dan, untuk installer, [Inno Setup 6](https://jrsoftware.org/isinfo.php). Repositori ini memakai `rust-lld` dan `llvm-dlltool` bawaan Rust, jadi tidak perlu Visual Studio Build Tools maupun MinGW. Sesuaikan path di `env.ps1` dengan lokasi Rust Anda, lalu muat dulu sebelum `cargo`:

```powershell
. .\env.ps1
cargo test
cargo run --release
```

Membuat installer (`dist\PDFCatcher-Setup-0.1.0.exe`):

```powershell
powershell -ExecutionPolicy Bypass -File .\build-installer.ps1
```

PDF contoh untuk uji manual: `cargo run --example make_fixtures` (hasil di `tests/fixtures`).

## Installer

- Per pengguna, **tanpa hak admin**.
- Folder instalasi bebas dipilih; default di drive lokal pertama selain C (misalnya `D:\PDF Catcher`).
- Memilih folder di drive C memunculkan peringatan, tapi tetap boleh.
- PC yang hanya punya drive C: dipasang di `%LOCALAPPDATA%\Programs\PDF Catcher` dengan pemberitahuan.
- Opsi shortcut Desktop dan menu klik kanan *Buka dengan PDF Catcher*.
- `config.json` disimpan di folder instalasi dan ikut terhapus saat uninstall.

## Lisensi

MIT. Pdfium dan komponennya memakai lisensi masing-masing (lihat `vendor/licenses`).
