// Build script PDF Catcher:
// 1. Menggambar logo (SVG) menjadi PNG berbagai ukuran.
// 2. Menulis berkas resource Windows (.res) berisi ikon dan meminta linker (rust-lld)
//    menyertakannya, tanpa butuh windres/rc.
// 3. Menulis assets/icon.ico untuk installer dan icon-256.png untuk ikon jendela.
// 4. Menyalin vendor/pdfium.dll ke folder target agar `cargo run` langsung jalan.

use std::path::{Path, PathBuf};

const LOGO: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="256" height="256">
<rect width="24" height="24" rx="5.5" fill="#D9352B"/>
<path d="M8 3.2h6.2l3.6 3.6v8.6a1 1 0 0 1-1 1H8a1 1 0 0 1-1-1V4.2a1 1 0 0 1 1-1z" fill="#FFFFFF"/>
<path d="M14.2 3.2v3.6h3.6z" fill="#F4B8B3"/>
<rect x="9" y="9" width="6.4" height="1.3" rx="0.65" fill="#D9352B"/>
<rect x="9" y="11.6" width="4.6" height="1.3" rx="0.65" fill="#D9352B"/>
<path d="M3.6 13.4c1.1 5.3 4.9 7.5 8.4 7.5s7.3-2.2 8.4-7.5" fill="none" stroke="#FFD34D" stroke-width="2.1" stroke-linecap="round"/>
</svg>"##;

fn render_png(size: u32) -> Vec<u8> {
    let tree = resvg::usvg::Tree::from_str(LOGO, &resvg::usvg::Options::default()).unwrap();
    let mut pm = resvg::tiny_skia::Pixmap::new(size, size).unwrap();
    let s = size as f32 / tree.size().width();
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(s, s), &mut pm.as_mut());
    // tiny-skia menyimpan warna premultiplied; PNG butuh straight alpha.
    let mut rgba = pm.data().to_vec();
    for px in rgba.chunks_mut(4) {
        let a = px[3] as f32 / 255.0;
        if a > 0.0 {
            for c in &mut px[..3] {
                *c = ((*c as f32 / a).round().min(255.0)) as u8;
            }
        }
    }
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, size, size);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header().unwrap().write_image_data(&rgba).unwrap();
    }
    out
}

fn res_entry(out: &mut Vec<u8>, ty: u16, name: u16, flags: u16, data: &[u8]) {
    let w32 = |o: &mut Vec<u8>, v: u32| o.extend_from_slice(&v.to_le_bytes());
    let w16 = |o: &mut Vec<u8>, v: u16| o.extend_from_slice(&v.to_le_bytes());
    w32(out, data.len() as u32); // DataSize
    w32(out, 32); // HeaderSize
    w16(out, 0xFFFF);
    w16(out, ty);
    w16(out, 0xFFFF);
    w16(out, name);
    w32(out, 0); // DataVersion
    w16(out, flags); // MemoryFlags
    w16(out, 0x0409); // LanguageId
    w32(out, 0); // Version
    w32(out, 0); // Characteristics
    out.extend_from_slice(data);
    while out.len() % 4 != 0 {
        out.push(0);
    }
}

fn icon_dir_entry(size: u32, bytes: usize, id_or_offset: u32, group: bool) -> Vec<u8> {
    let d = if size >= 256 { 0u8 } else { size as u8 };
    let mut e = vec![d, d, 0, 0];
    e.extend_from_slice(&1u16.to_le_bytes()); // planes
    e.extend_from_slice(&32u16.to_le_bytes()); // bit count
    e.extend_from_slice(&(bytes as u32).to_le_bytes());
    if group {
        e.extend_from_slice(&(id_or_offset as u16).to_le_bytes());
    } else {
        e.extend_from_slice(&id_or_offset.to_le_bytes());
    }
    e
}

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let sizes = [16u32, 24, 32, 48, 64, 128, 256];
    let pngs: Vec<Vec<u8>> = sizes.iter().map(|s| render_png(*s)).collect();

    // --- .res untuk ikon exe ---
    let mut res = Vec::new();
    res_entry(&mut res, 0, 0, 0, &[]); // entri kosong pembuka berkas .res
    for (i, p) in pngs.iter().enumerate() {
        res_entry(&mut res, 3, (i + 1) as u16, 0x1010, p); // RT_ICON
    }
    let mut grp = Vec::new();
    grp.extend_from_slice(&0u16.to_le_bytes());
    grp.extend_from_slice(&1u16.to_le_bytes());
    grp.extend_from_slice(&(sizes.len() as u16).to_le_bytes());
    for (i, s) in sizes.iter().enumerate() {
        grp.extend(icon_dir_entry(*s, pngs[i].len(), (i + 1) as u32, true));
    }
    res_entry(&mut res, 14, 1, 0x1030, &grp); // RT_GROUP_ICON
    let res_path = out_dir.join("pdfcatcher-icon.res");
    std::fs::write(&res_path, res).unwrap();
    println!("cargo:rustc-link-arg-bins={}", res_path.display());

    // --- icon.ico untuk installer ---
    let mut ico = Vec::new();
    ico.extend_from_slice(&0u16.to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&(sizes.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * sizes.len() as u32;
    for (i, s) in sizes.iter().enumerate() {
        ico.extend(icon_dir_entry(*s, pngs[i].len(), offset, false));
        offset += pngs[i].len() as u32;
    }
    for p in &pngs {
        ico.extend_from_slice(p);
    }
    let assets = manifest.join("assets");
    let _ = std::fs::create_dir_all(&assets);
    write_if_changed(&assets.join("icon.ico"), &ico);

    // --- PNG untuk ikon jendela ---
    std::fs::write(out_dir.join("icon-256.png"), &pngs[6]).unwrap();

    // --- salin pdfium.dll ke target/<profil>/ ---
    let dll = manifest.join("vendor").join("pdfium.dll");
    if let Some(profile_dir) = out_dir.ancestors().nth(3) {
        if dll.exists() {
            let dest = profile_dir.join("pdfium.dll");
            let need = match (std::fs::metadata(&dll), std::fs::metadata(&dest)) {
                (Ok(a), Ok(b)) => a.len() != b.len(),
                _ => true,
            };
            if need {
                let _ = std::fs::copy(&dll, &dest);
            }
        }
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=vendor/pdfium.dll");
}

fn write_if_changed(path: &Path, data: &[u8]) {
    if std::fs::read(path).map(|old| old != data).unwrap_or(true) {
        std::fs::write(path, data).unwrap();
    }
}
