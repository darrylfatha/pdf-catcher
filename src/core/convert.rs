//! Konversi PDF ke DOCX: ambil teks + gambar per halaman, susun menjadi paragraf,
//! lalu tulis dengan docx-rs.

use std::io::Cursor;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};

use docx_rs::{BreakType, Docx, Paragraph, Pic, Run};
use pdfium_render::prelude::*;

use super::layout::{self, Frag, Para};
use super::{open_pdf, output, pdf_err, ranges, Reporter};
use crate::error::AppError;

/// Lebar area tulis A4 dengan margin 1 inci, dalam point.
const TEXT_WIDTH_PT: f32 = 451.0;
const TEXT_HEIGHT_PT: f32 = 650.0;
const EMU_PER_PT: f32 = 12_700.0;

enum Block {
    Text(Para),
    Image { png: Vec<u8>, w_pt: f32, h_pt: f32, top: f32 },
}

impl Block {
    fn top(&self) -> f32 {
        match self {
            Block::Text(p) => p.top,
            Block::Image { top, .. } => *top,
        }
    }
}

/// Mengubah PDF (semua halaman, atau rentang tertentu) menjadi satu file DOCX di `output`.
pub fn pdf_to_docx(
    pdfium: &Pdfium,
    input: &Path,
    pages: Option<&[RangeInclusive<usize>]>,
    output: &Path,
    rep: &mut Reporter,
) -> Result<PathBuf, AppError> {
    let doc = open_pdf(pdfium, input)?;
    let page_count = doc.pages().len() as usize;
    let list: Vec<usize> = match pages {
        Some(r) => ranges::page_list(r).into_iter().filter(|p| *p >= 1 && *p <= page_count).collect(),
        None => (1..=page_count).collect(),
    };
    if list.is_empty() {
        return Err(AppError::BadRange("tidak ada halaman yang dipilih".into()));
    }

    let mut per_page: Vec<Vec<Block>> = Vec::with_capacity(list.len());
    let mut total_chars = 0usize;

    for (k, number) in list.iter().enumerate() {
        let page = doc.pages().get((number - 1) as PdfPageIndex).map_err(pdf_err)?;
        let mut blocks = Vec::new();

        // Teks
        if let Ok(text) = page.text() {
            let mut frags = Vec::new();
            for seg in text.segments().iter() {
                let s = seg.text();
                if s.trim().is_empty() {
                    continue;
                }
                total_chars += s.trim().chars().count();
                let b = seg.bounds();
                let (size, bold) = segment_style(&seg);
                frags.push(Frag {
                    text: s,
                    left: b.left().value,
                    right: b.right().value,
                    top: b.top().value,
                    bottom: b.bottom().value,
                    size: if size > 0.0 { size } else { (b.top().value - b.bottom().value).abs() },
                    bold,
                });
            }
            let lines = layout::to_lines(frags);
            blocks.extend(layout::to_paragraphs(&lines).into_iter().map(Block::Text));
        }

        // Gambar
        for obj in page.objects().iter() {
            let Some(img) = obj.as_image_object() else { continue };
            let Ok(bounds) = obj.bounds() else { continue };
            let Ok(raw) = img.get_raw_image() else { continue };
            let rgba = image::DynamicImage::ImageRgba8(raw.to_rgba8());
            let mut png = Vec::new();
            if rgba.write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png).is_err() {
                continue;
            }
            let w = (bounds.right().value - bounds.left().value).abs();
            let h = (bounds.top().value - bounds.bottom().value).abs();
            if w < 4.0 || h < 4.0 {
                continue;
            }
            blocks.push(Block::Image { png, w_pt: w, h_pt: h, top: bounds.top().value });
        }

        blocks.sort_by(|a, b| b.top().total_cmp(&a.top()));
        per_page.push(blocks);
        rep.step(k + 1, list.len() + 1)?;
    }

    if total_chars == 0 {
        return Err(AppError::NoText(input.to_path_buf()));
    }

    // Ukuran isi dihitung dari seluruh dokumen agar judul terdeteksi konsisten.
    let all_paras: Vec<Para> = per_page
        .iter()
        .flatten()
        .filter_map(|b| match b {
            Block::Text(p) => Some(p.clone()),
            _ => None,
        })
        .collect();
    let body = layout::body_size(&all_paras);

    let mut docx = Docx::new();
    let last = per_page.len().saturating_sub(1);
    for (i, blocks) in per_page.into_iter().enumerate() {
        for block in blocks {
            match block {
                Block::Text(p) => {
                    let pt = p.size.clamp(6.0, 72.0);
                    let mut run = Run::new().add_text(&p.text).size((pt * 2.0).round() as usize);
                    if p.bold || layout::is_heading(&p, body) {
                        run = run.bold();
                    }
                    docx = docx.add_paragraph(Paragraph::new().add_run(run));
                }
                Block::Image { png, w_pt, h_pt, .. } => {
                    let scale = (TEXT_WIDTH_PT / w_pt).min(TEXT_HEIGHT_PT / h_pt).min(1.0);
                    let w = (w_pt * scale * EMU_PER_PT) as u32;
                    let h = (h_pt * scale * EMU_PER_PT) as u32;
                    let pic = Pic::new(&png).size(w, h);
                    docx = docx.add_paragraph(Paragraph::new().add_run(Run::new().add_image(pic)));
                }
            }
        }
        if i < last {
            docx = docx.add_paragraph(Paragraph::new().add_run(Run::new().add_break(BreakType::Page)));
        }
    }

    rep.check()?;
    let tmp = output::temp_for(output);
    let write = || -> Result<(), AppError> {
        let file = std::fs::File::create(&tmp)?;
        docx.build()
            .pack(file)
            .map_err(|e| AppError::Pdf(format!("gagal menulis DOCX: {e}")))?;
        Ok(())
    };
    if let Err(e) = write() {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    rep.step(list.len() + 1, list.len() + 1).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })?;
    output::commit(&tmp, output)?;
    Ok(output.to_path_buf())
}

/// Ukuran huruf dan ketebalan sebuah segmen teks, dibaca dari karakter pertamanya.
fn segment_style(seg: &PdfPageTextSegment) -> (f32, bool) {
    let Ok(chars) = seg.chars() else { return (0.0, false) };
    for ch in chars.iter() {
        if ch.unicode_char().map(|c| c.is_whitespace()).unwrap_or(true) {
            continue;
        }
        let size = ch.scaled_font_size().value;
        let bold = matches!(
            ch.font_weight(),
            Some(PdfFontWeight::Weight600 | PdfFontWeight::Weight700Bold | PdfFontWeight::Weight800 | PdfFontWeight::Weight900)
        ) || matches!(ch.font_weight(), Some(PdfFontWeight::Custom(w)) if w >= 600)
            || ch.font_name().to_lowercase().contains("bold");
        return (size, bold);
    }
    (0.0, false)
}
