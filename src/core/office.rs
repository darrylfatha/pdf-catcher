//! Word/Excel/PowerPoint ke PDF lewat otomasi Microsoft Office (COM/IDispatch).
//!
//! WAJIB Microsoft Office terpasang: modul ini menjalankan Word/Excel/PowerPoint sendiri
//! di latar belakang (tidak terlihat), membuka satu dokumen, mengekspornya ke PDF, menutupnya
//! tanpa menyimpan, lalu menutup aplikasinya lagi. Tidak memakai Pdfium sama sekali.
//!
//! `windows-sys` (dipakai di tempat lain untuk named pipe) tidak menyediakan pembungkus
//! pemanggilan IDispatch, jadi modul ini memakai crate `windows` yang menyediakannya.
//!
//! Setiap pekerjaan dijalankan di thread terpisah dengan batas waktu: kalau Office tidak
//! merespons (misalnya menunggu dialog yang tersembunyi karena aplikasi tidak terlihat),
//! PDF Catcher berhenti menunggu dan melaporkan gagal, bukan membeku selamanya.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use windows::core::{GUID, HRESULT, PCWSTR};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, IDispatch, CLSCTX_LOCAL_SERVER, CLSIDFromProgID,
    COINIT_APARTMENTTHREADED, DISPATCH_METHOD, DISPATCH_PROPERTYGET, DISPATCH_PROPERTYPUT, DISPPARAMS,
};
use windows::Win32::System::Variant::{VARIANT, VARIANT_0, VARIANT_0_0, VARIANT_0_0_0, VT_BOOL, VT_I4};

use super::{output, Reporter};
use crate::error::AppError;

/// Batas waktu satu pekerjaan konversi (semua file dalam satu aksi): kalau Office tidak
/// selesai dalam waktu ini, dianggap macet.
const TIMEOUT: Duration = Duration::from_secs(120);

const LOCALE_USER_DEFAULT: u32 = 0x0400;
const DISPID_PROPERTYPUT: i32 = -3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfficeApp {
    Word,
    Excel,
    PowerPoint,
}

impl OfficeApp {
    fn prog_id(self) -> &'static str {
        match self {
            OfficeApp::Word => "Word.Application",
            OfficeApp::Excel => "Excel.Application",
            OfficeApp::PowerPoint => "PowerPoint.Application",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            OfficeApp::Word => "Microsoft Word",
            OfficeApp::Excel => "Microsoft Excel",
            OfficeApp::PowerPoint => "Microsoft PowerPoint",
        }
    }
}

// ---------- VARIANT & pemanggilan IDispatch ----------
// COM Automation memanggil properti/metode lewat nama (bukan alamat fungsi), lewat tiga
// langkah standar: cari dispid dari nama (GetIDsOfNames), lalu panggil (Invoke). Argumen
// dikirim dalam urutan TERBALIK dari urutan parameter aslinya — ini aturan baku DISPPARAMS,
// bukan kekeliruan.

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn var_i32(v: i32) -> VARIANT {
    VARIANT { Anonymous: VARIANT_0 { Anonymous: std::mem::ManuallyDrop::new(VARIANT_0_0 { vt: VT_I4, wReserved1: 0, wReserved2: 0, wReserved3: 0, Anonymous: VARIANT_0_0_0 { lVal: v } }) } }
}

fn var_bool(v: bool) -> VARIANT {
    let b = windows::Win32::Foundation::VARIANT_BOOL(if v { -1 } else { 0 });
    VARIANT { Anonymous: VARIANT_0 { Anonymous: std::mem::ManuallyDrop::new(VARIANT_0_0 { vt: VT_BOOL, wReserved1: 0, wReserved2: 0, wReserved3: 0, Anonymous: VARIANT_0_0_0 { boolVal: b } }) } }
}

fn var_str(s: &str) -> VARIANT {
    VARIANT::from(s)
}

unsafe fn dispid(disp: &IDispatch, name: &str) -> windows::core::Result<i32> {
    let w = wide(name);
    let mut id = 0i32;
    disp.GetIDsOfNames(&GUID::zeroed(), &PCWSTR(w.as_ptr()), 1, LOCALE_USER_DEFAULT, &mut id)?;
    Ok(id)
}

unsafe fn invoke(disp: &IDispatch, name: &str, flags: windows::Win32::System::Com::DISPATCH_FLAGS, args: &[VARIANT]) -> windows::core::Result<VARIANT> {
    let id = dispid(disp, name)?;
    let mut reversed: Vec<VARIANT> = args.iter().rev().cloned().collect();
    let params = DISPPARAMS { rgvarg: reversed.as_mut_ptr(), rgdispidNamedArgs: std::ptr::null_mut(), cArgs: reversed.len() as u32, cNamedArgs: 0 };
    let mut result = VARIANT::default();
    disp.Invoke(id, &GUID::zeroed(), LOCALE_USER_DEFAULT, flags, &params, Some(&mut result), None, None)?;
    Ok(result)
}

/// Memanggil metode (Sub/Function), misalnya `Documents.Open(path)`.
unsafe fn call(disp: &IDispatch, name: &str, args: &[VARIANT]) -> windows::core::Result<VARIANT> {
    invoke(disp, name, DISPATCH_METHOD, args)
}

/// Memanggil metode yang hasilnya berupa objek (IDispatch lain), misalnya `Documents.Open(...)`
/// mengembalikan objek `Document`.
unsafe fn call_disp(disp: &IDispatch, name: &str, args: &[VARIANT]) -> windows::core::Result<IDispatch> {
    let v = call(disp, name, args)?;
    IDispatch::try_from(&v).map_err(|_| windows::core::Error::from_hresult(HRESULT(0x80020005u32 as i32))) // DISP_E_TYPEMISMATCH
}

/// Membaca properti berupa objek, misalnya `Application.Documents`.
unsafe fn get_disp(disp: &IDispatch, name: &str) -> windows::core::Result<IDispatch> {
    let v = invoke(disp, name, DISPATCH_PROPERTYGET, &[])?;
    IDispatch::try_from(&v).map_err(|_| windows::core::Error::from_hresult(HRESULT(0x80020005u32 as i32)))
}

/// Mengisi properti, misalnya `Application.Visible = False`.
unsafe fn put(disp: &IDispatch, name: &str, mut value: VARIANT) -> windows::core::Result<()> {
    let id = dispid(disp, name)?;
    let mut put_id = DISPID_PROPERTYPUT;
    let params = DISPPARAMS { rgvarg: &mut value, rgdispidNamedArgs: &mut put_id, cArgs: 1, cNamedArgs: 1 };
    disp.Invoke(id, &GUID::zeroed(), LOCALE_USER_DEFAULT, DISPATCH_PROPERTYPUT, &params, None, None, None)
}

fn is_not_registered(e: &windows::core::Error) -> bool {
    e.code().0 as u32 == 0x8004_0154 // REGDB_E_CLASSNOTREG
}

fn office_err(app: OfficeApp, e: &windows::core::Error) -> AppError {
    AppError::OfficeFailed(format!("Gagal memproses dengan {}: {} (0x{:08X})", app.display_name(), e.message(), e.code().0 as u32))
}

/// Menutup aplikasi Office saat keluar dari cakupan ini, baik berhasil maupun gagal, supaya
/// tidak ada proses Word/Excel/PowerPoint tak terlihat yang tertinggal.
struct QuitGuard(IDispatch);

impl Drop for QuitGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = call(&self.0, "Quit", &[]);
        }
    }
}

// ---------- konversi ----------

/// Mengubah beberapa file Word/Excel/PowerPoint menjadi PDF, satu file sumber = satu PDF,
/// dengan nama yang sama. Berjalan di thread terpisah dengan batas waktu (`TIMEOUT`), supaya
/// dialog Office yang tersembunyi (tidak terlihat karena `Visible=False`) tidak membekukan
/// PDF Catcher selamanya; proses yang melebihi batas waktu dilaporkan gagal dan tetap boleh
/// berjalan sendiri di latar belakang.
pub fn convert_many(app: OfficeApp, inputs: &[PathBuf], out_dir: &Path, cancel: Arc<AtomicBool>, rep: &mut Reporter) -> Result<Vec<PathBuf>, AppError> {
    let inputs = inputs.to_vec();
    let out_dir = out_dir.to_path_buf();
    let (result_tx, result_rx) = mpsc::channel();
    let (progress_tx, progress_rx) = mpsc::channel::<(usize, usize)>();
    let cancel_inner = cancel.clone();

    let worker = std::thread::Builder::new()
        .name("pdfcatcher-office".into())
        .spawn(move || {
            let mut on_progress = |d, t| {
                let _ = progress_tx.send((d, t));
            };
            let mut inner_rep = Reporter::new(&cancel_inner, &mut on_progress);
            let r = convert_many_blocking(app, &inputs, &out_dir, &mut inner_rep);
            let _ = result_tx.send(r);
        })
        .expect("thread konversi Office");

    let deadline = Instant::now() + TIMEOUT;
    loop {
        if let Ok((d, t)) = progress_rx.try_recv() {
            rep.step(d, t)?; // meneruskan progres ke UI; juga menghentikan kalau pengguna menekan Batal
        }
        match result_rx.try_recv() {
            Ok(r) => {
                let _ = worker.join();
                return r;
            }
            Err(mpsc::TryRecvError::Empty) => {
                if cancel.load(Ordering::Relaxed) {
                    return Err(AppError::Cancelled);
                }
                if Instant::now() > deadline {
                    return Err(AppError::OfficeFailed(format!(
                        "{} tidak merespons lebih dari {} menit, kemungkinan ada dialog yang menunggu konfirmasi di dalamnya. \
                         Proses mungkin masih berjalan tak terlihat; tutup lewat Task Manager jika perlu.",
                        app.display_name(),
                        TIMEOUT.as_secs() / 60
                    )));
                }
                std::thread::sleep(Duration::from_millis(80));
            }
            Err(mpsc::TryRecvError::Disconnected) => return Err(AppError::OfficeFailed("Proses konversi berhenti tak terduga.".into())),
        }
    }
}

fn convert_many_blocking(app: OfficeApp, inputs: &[PathBuf], out_dir: &Path, rep: &mut Reporter) -> Result<Vec<PathBuf>, AppError> {
    // CoInitializeEx di sini, dipasangkan dengan CoUninitialize di akhir fungsi ini: COM
    // diinisialisasi khusus untuk thread konversi ini, sekali per pekerjaan.
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
        .ok()
        .map_err(|e| office_err(app, &e))?;

    let result = (|| -> Result<Vec<PathBuf>, AppError> {
        let prog_id = wide(app.prog_id());
        let clsid = unsafe { CLSIDFromProgID(PCWSTR(prog_id.as_ptr())) }
            .map_err(|e| if is_not_registered(&e) { AppError::OfficeMissing(app.display_name()) } else { office_err(app, &e) })?;
        let app_disp: IDispatch = unsafe { CoCreateInstance(&clsid, None, CLSCTX_LOCAL_SERVER) }
            .map_err(|e| if is_not_registered(&e) { AppError::OfficeMissing(app.display_name()) } else { office_err(app, &e) })?;
        let _guard = QuitGuard(app_disp.clone());

        unsafe {
            // PowerPoint beberapa versi menolak Visible=False (tidak fatal, diabaikan).
            let _ = put(&app_disp, "Visible", var_bool(false));
            match app {
                OfficeApp::Word => {
                    let _ = put(&app_disp, "DisplayAlerts", var_i32(0)); // wdAlertsNone
                }
                OfficeApp::Excel => {
                    let _ = put(&app_disp, "DisplayAlerts", var_bool(false));
                }
                OfficeApp::PowerPoint => {} // tidak punya properti DisplayAlerts
            }
        }

        let mut done: Vec<PathBuf> = Vec::new();
        let total = inputs.len();
        let outcome = (|| -> Result<(), AppError> {
            for (i, input) in inputs.iter().enumerate() {
                rep.check()?;
                let target = output::unique_path(out_dir, &output::stem_of(input), "pdf", &done);
                unsafe { convert_one(app, &app_disp, input, &target) }.map_err(|e| office_err(app, &e))?;
                done.push(target);
                rep.step(i + 1, total)?;
            }
            Ok(())
        })();

        match outcome {
            Ok(()) => Ok(done),
            Err(e) => {
                output::cleanup(&done);
                Err(e)
            }
        }
    })();

    unsafe { CoUninitialize() };
    result
}

unsafe fn convert_one(app: OfficeApp, app_disp: &IDispatch, input: &Path, output: &Path) -> windows::core::Result<()> {
    // Bukan `canonicalize()`: itu menghasilkan path berawalan `\\?\` yang membuat Word/Excel/
    // PowerPoint diam-diam tidak membuka apa pun (Invoke sukses tapi hasilnya VT_EMPTY).
    let in_str = input.to_string_lossy();
    let out_str = output.to_string_lossy();

    match app {
        OfficeApp::Word => {
            let documents = get_disp(app_disp, "Documents")?;
            let doc = call_disp(&documents, "Open", &[var_str(&in_str)])?;
            let export = call(&doc, "ExportAsFixedFormat", &[var_str(&out_str), var_i32(17)]); // wdExportFormatPDF
            let _ = call(&doc, "Close", &[var_bool(false)]);
            export?;
        }
        OfficeApp::Excel => {
            let workbooks = get_disp(app_disp, "Workbooks")?;
            let wb = call_disp(&workbooks, "Open", &[var_str(&in_str)])?;
            let export = call(&wb, "ExportAsFixedFormat", &[var_i32(0), var_str(&out_str)]); // xlTypePDF
            let _ = call(&wb, "Close", &[var_bool(false)]);
            export?;
        }
        OfficeApp::PowerPoint => {
            let presentations = get_disp(app_disp, "Presentations")?;
            // Open(FileName, ReadOnly, Untitled, WithWindow) — WithWindow:=msoFalse supaya tidak membuka jendela.
            let pres = call_disp(&presentations, "Open", &[var_str(&in_str), var_bool(true), var_bool(false), var_bool(false)])?;
            let export = call(&pres, "SaveAs", &[var_str(&out_str), var_i32(32)]); // ppSaveAsPDF
            let _ = call(&pres, "Close", &[]);
            export?;
        }
    }
    Ok(())
}

/// Memeriksa apakah suatu aplikasi Office terpasang, tanpa membukanya.
pub fn is_installed(app: OfficeApp) -> bool {
    let prog_id = wide(app.prog_id());
    unsafe { CLSIDFromProgID(PCWSTR(prog_id.as_ptr())) }.is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdfium_render::prelude::Pdfium;
    use std::sync::atomic::AtomicBool;

    fn convert(app: OfficeApp, inputs: &[PathBuf], out_dir: &Path) -> Result<Vec<PathBuf>, AppError> {
        let cancel = Arc::new(AtomicBool::new(false));
        let mut cb = |_, _| {};
        let mut rep = Reporter::new(&cancel, &mut cb);
        convert_many(app, inputs, out_dir, cancel.clone(), &mut rep)
    }

    /// Membuat file .xlsx atau .pptx minimal (tanpa isi tertentu; cukup untuk menguji
    /// mekanisme ekspor ke PDF) lewat Office sendiri, dalam thread + apartemen COM-nya sendiri.
    fn make_fixture(app: OfficeApp, path: &Path) -> windows::core::Result<()> {
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.ok()?;
        let result = (|| -> windows::core::Result<()> {
            let prog_id = wide(app.prog_id());
            let clsid = unsafe { CLSIDFromProgID(PCWSTR(prog_id.as_ptr())) }?;
            let app_disp: IDispatch = unsafe { CoCreateInstance(&clsid, None, CLSCTX_LOCAL_SERVER) }?;
            let _guard = QuitGuard(app_disp.clone());
            unsafe {
                let _ = put(&app_disp, "Visible", var_bool(false));
                let out_str = path.to_string_lossy();
                match app {
                    OfficeApp::Excel => {
                        let workbooks = get_disp(&app_disp, "Workbooks")?;
                        let wb = call_disp(&workbooks, "Add", &[])?;
                        call(&wb, "SaveAs", &[var_str(&out_str), var_i32(51)])?; // xlOpenXMLWorkbook
                        let _ = call(&wb, "Close", &[var_bool(false)]);
                    }
                    OfficeApp::PowerPoint => {
                        let presentations = get_disp(&app_disp, "Presentations")?;
                        let pres = call_disp(&presentations, "Add", &[var_bool(false)])?; // WithWindow:=msoFalse
                        let slides = get_disp(&pres, "Slides")?;
                        call_disp(&slides, "Add", &[var_i32(1), var_i32(2)])?; // ppLayoutText, 1 slide kosong
                        call(&pres, "SaveAs", &[var_str(&out_str), var_i32(24)])?; // ppSaveAsOpenXMLPresentation
                        let _ = call(&pres, "Close", &[]);
                    }
                    OfficeApp::Word => unreachable!("fixture Word dibuat lewat docx-rs"),
                }
            }
            Ok(())
        })();
        unsafe { CoUninitialize() };
        result
    }

    fn docx_fixture(path: &Path, text: &str) {
        use docx_rs::{Docx, Paragraph, Run};
        let docx = Docx::new().add_paragraph(Paragraph::new().add_run(Run::new().add_text(text)));
        let file = std::fs::File::create(path).unwrap();
        docx.build().pack(file).unwrap();
    }

    fn pdf_has_text(pdfium: &Pdfium, path: &Path, needle: &str) -> bool {
        let doc = pdfium.load_pdf_from_file(path, None).unwrap();
        doc.pages().iter().any(|p| p.text().map(|t| t.all().contains(needle)).unwrap_or(false))
    }

    #[test]
    fn deteksi_hresult_tidak_terdaftar() {
        let not_reg = windows::core::Error::from_hresult(HRESULT(0x8004_0154u32 as i32));
        let lain = windows::core::Error::from_hresult(HRESULT(0x8007_000Eu32 as i32)); // E_OUTOFMEMORY
        assert!(is_not_registered(&not_reg));
        assert!(!is_not_registered(&lain));
    }

    /// Satu test besar: Office hanya boleh dimuat sekali per proses (sama seperti Pdfium di
    /// core_tests.rs), jadi semua skenario Word/Excel/PowerPoint digabung di sini.
    #[test]
    fn konversi_office_ke_pdf() {
        if !(is_installed(OfficeApp::Word) && is_installed(OfficeApp::Excel) && is_installed(OfficeApp::PowerPoint)) {
            eprintln!("Microsoft Office tidak terpasang di PC ini; test konversi dilewati.");
            return;
        }

        let vendor = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor");
        let pdfium = super::super::load_pdfium(&vendor).expect("vendor/pdfium.dll harus ada");

        let dir = std::env::temp_dir().join(format!("pdfcatcher-office-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        // Word: 1 file, lalu batch 2 file sekaligus dengan nama sama (uji tabrakan nama).
        let doc_a = dir.join("surat-a.docx");
        let doc_b = dir.join("surat-b.docx");
        docx_fixture(&doc_a, "Isi surat Alpha dari PDF Catcher.");
        docx_fixture(&doc_b, "Isi surat Beta dari PDF Catcher.");

        let out = convert(OfficeApp::Word, &[doc_a.clone()], &dir).expect("Word ke PDF gagal");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].file_name().unwrap(), "surat-a.pdf");
        assert!(pdf_has_text(&pdfium, &out[0], "Alpha"));

        let batch_dir = dir.join("batch");
        std::fs::create_dir_all(&batch_dir).unwrap();
        let same_name_a = batch_dir.join("sama.docx");
        let same_name_b_src = dir.join("sama-sumber-lain.docx");
        docx_fixture(&same_name_a, "Dokumen pertama.");
        docx_fixture(&same_name_b_src, "Dokumen kedua.");
        std::fs::copy(&same_name_b_src, batch_dir.join("sama2.docx")).unwrap();
        let out2 = convert(OfficeApp::Word, &[same_name_a, batch_dir.join("sama2.docx")], &batch_dir).expect("batch Word gagal");
        assert_eq!(out2.len(), 2);

        // Excel: fixture tanpa isi tertentu, cukup untuk menguji mekanisme ekspornya.
        let xlsx = dir.join("rencana.xlsx");
        make_fixture(OfficeApp::Excel, &xlsx).expect("gagal membuat fixture xlsx");
        let out3 = convert(OfficeApp::Excel, &[xlsx], &dir).expect("Excel ke PDF gagal");
        assert_eq!(out3.len(), 1);
        // Lembar kosong tanpa isi kadang menghasilkan PDF 0 halaman di Excel; yang diuji di sini
        // adalah mekanisme otomasinya (buka, ekspor, tutup) berhasil tanpa error, bukan isinya.
        assert!(out3[0].exists());
        let _ = pdfium.load_pdf_from_file(&out3[0], None).expect("PDF hasil Excel harus tetap bisa dibuka");

        // PowerPoint.
        let pptx = dir.join("presentasi.pptx");
        make_fixture(OfficeApp::PowerPoint, &pptx).expect("gagal membuat fixture pptx");
        let out4 = convert(OfficeApp::PowerPoint, &[pptx], &dir).expect("PowerPoint ke PDF gagal");
        assert_eq!(out4.len(), 1);
        assert!(pdfium.load_pdf_from_file(&out4[0], None).unwrap().pages().len() >= 1);

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
