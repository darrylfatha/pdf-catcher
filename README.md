# PDF Catcher

Aplikasi desktop Windows untuk **menangkap** file (PDF, Word, Excel, PowerPoint, JPG/PNG) lalu mengolahnya
dari atau ke PDF. Semua proses berjalan di komputer sendiri, tanpa internet dan tanpa unggah file.

Dibuat dengan Rust + egui (UI native) dan Pdfium (mesin PDF milik Chrome). Konversi Word/Excel/PowerPoint
ke PDF berjalan lewat Microsoft Office sendiri (otomasi COM), jadi membutuhkan Office terpasang.

## Cara pakai

1. **Tangkap file.** Seret PDF/Word/Excel/PowerPoint/JPG/PNG (atau satu folder) ke jendela, klik kotak tangkap, atau klik kanan file lalu pilih *Buka dengan PDF Catcher*.
2. **Pilih aksi.** Dari PDF: Gabung (2 PDF atau lebih), Pisah (1 PDF), Ke Word (1 PDF). Ke PDF: Gambar ke PDF (1 JPG/PNG atau lebih, digabung jadi satu), Word/Excel/PowerPoint ke PDF (1 file atau lebih sekaligus, tiap file jadi satu PDF).
3. **Atur & simpan.** Hasil disimpan di folder file asli (default), atau di folder lain lewat *Ganti folder…*. Nama yang sudah dipakai otomatis diberi akhiran (1), (2); file asli tidak pernah ditimpa.

Folder hasil default bisa diubah di **Pengaturan**: sama dengan folder file asli, satu folder pilihan, atau tanya setiap kali.

## Word/Excel/PowerPoint ke PDF

Fitur ini menjalankan Word/Excel/PowerPoint sendiri sebentar di latar belakang (tidak terlihat) untuk
membuka file, mengekspornya ke PDF, lalu menutupnya lagi — **wajib Microsoft Office terpasang**. Kalau
tidak ada, PDF Catcher memberi tahu dengan jelas dan fitur PDF lainnya tetap jalan normal. Ada batas waktu
2 menit per pekerjaan: kalau Office tidak merespons (misalnya ada dialog tersembunyi yang menunggu
konfirmasi), PDF Catcher berhenti menunggu dan melaporkan gagal, bukan membeku selamanya.

## Struktur

```
src/
  main.rs, app.rs        jendela, state, alur tangkap -> aksi -> simpan
  ui/                    tampilan (zona tangkap, keranjang, kartu aksi, simpan ke, pengaturan)
  lib.rs                 logika tanpa UI (bisa diuji)
  core/                  intake, merge, split, ranges, output, convert, layout, images, office
  worker.rs              satu thread pekerja yang memegang Pdfium (+ thread sendiri per konversi Office)
  settings.rs            config.json
  single_instance.rs     satu jendela (named pipe)
installer/pdfcatcher.iss skrip Inno Setup
vendor/pdfium.dll        dari github.com/bblanchon/pdfium-binaries (win-x64)
tests/                   test dengan Pdfium & Microsoft Office sungguhan + file contoh
```

## Build dari sumber

Butuh Rust (toolchain `x86_64-pc-windows-gnu`) dan, untuk installer, [Inno Setup 6](https://jrsoftware.org/isinfo.php). Repositori ini memakai `rust-lld` dan `llvm-dlltool` bawaan Rust, jadi tidak perlu Visual Studio Build Tools maupun MinGW. Sesuaikan path di `env.ps1` dengan lokasi Rust Anda, lalu muat dulu sebelum `cargo`:

```powershell
. .\env.ps1
cargo test
cargo run --release
```

Membuat installer (`dist\PDFCatcher-Setup-1.1.0.exe`):

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
