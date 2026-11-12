//! Menyusun potongan teks PDF (yang hanya punya posisi) menjadi baris lalu paragraf.
//! Koordinat mengikuti PDF: sumbu y naik ke atas, satuan point.

/// Potongan teks dengan kotak pembatasnya.
#[derive(Debug, Clone)]
pub struct Frag {
    pub text: String,
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
    pub size: f32,
    pub bold: bool,
}

#[derive(Debug, Clone)]
pub struct Line {
    pub text: String,
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
    pub size: f32,
    pub bold: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Para {
    pub text: String,
    pub size: f32,
    pub bold: bool,
    /// Posisi atas paragraf, untuk mengurutkan dengan gambar.
    pub top: f32,
}

fn center(top: f32, bottom: f32) -> f32 {
    (top + bottom) / 2.0
}

/// Mengelompokkan potongan teks menjadi baris berdasarkan posisi vertikal yang berdekatan.
pub fn to_lines(mut frags: Vec<Frag>) -> Vec<Line> {
    frags.retain(|f| !f.text.trim().is_empty());
    frags.sort_by(|a, b| {
        center(b.top, b.bottom)
            .total_cmp(&center(a.top, a.bottom))
            .then(a.left.total_cmp(&b.left))
    });

    let mut groups: Vec<Vec<Frag>> = Vec::new();
    for f in frags {
        let c = center(f.top, f.bottom);
        let h = (f.top - f.bottom).abs().max(1.0);
        if let Some(g) = groups.last_mut() {
            let gt = g.iter().map(|x| x.top).fold(f32::MIN, f32::max);
            let gb = g.iter().map(|x| x.bottom).fold(f32::MAX, f32::min);
            let gh = (gt - gb).abs().max(1.0);
            if (c - center(gt, gb)).abs() < 0.5 * h.max(gh) {
                g.push(f);
                continue;
            }
        }
        groups.push(vec![f]);
    }

    groups
        .into_iter()
        .map(|mut g| {
            g.sort_by(|a, b| a.left.total_cmp(&b.left));
            let mut text = String::new();
            let mut prev_right: Option<f32> = None;
            for f in &g {
                if let Some(pr) = prev_right {
                    let gap = f.left - pr;
                    if gap > 0.15 * f.size && !text.ends_with(' ') && !f.text.starts_with(' ') {
                        text.push(' ');
                    }
                }
                text.push_str(&f.text);
                prev_right = Some(f.right);
            }
            let chars: usize = g.iter().map(|f| f.text.chars().count()).sum::<usize>().max(1);
            let size = g.iter().map(|f| f.size * f.text.chars().count() as f32).sum::<f32>() / chars as f32;
            let bold_chars: usize = g.iter().filter(|f| f.bold).map(|f| f.text.chars().count()).sum();
            Line {
                text: collapse_spaces(&text),
                left: g.iter().map(|f| f.left).fold(f32::MAX, f32::min),
                right: g.iter().map(|f| f.right).fold(f32::MIN, f32::max),
                top: g.iter().map(|f| f.top).fold(f32::MIN, f32::max),
                bottom: g.iter().map(|f| f.bottom).fold(f32::MAX, f32::min),
                size,
                bold: bold_chars * 2 > chars,
            }
        })
        .filter(|l| !l.text.is_empty())
        .collect()
}

fn collapse_spaces(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut space = false;
    for c in s.trim().chars() {
        if c.is_whitespace() {
            if !space {
                out.push(' ');
            }
            space = true;
        } else {
            out.push(c);
            space = false;
        }
    }
    out
}

/// Menggabungkan baris menjadi paragraf.
/// Baris baru memulai paragraf jika jaraknya ke baris sebelumnya jauh lebih besar dari
/// biasanya, indentasinya bertambah, atau ukuran/ketebalan hurufnya berubah.
pub fn to_paragraphs(lines: &[Line]) -> Vec<Para> {
    let mut paras: Vec<Para> = Vec::new();
    let mut prev: Option<&Line> = None;
    let mut lines_in_para = 0usize;

    for line in lines {
        let start_new = match prev {
            None => true,
            Some(p) => {
                let lh = (p.top - p.bottom).abs().max(1.0);
                let gap = p.bottom - line.top;
                let size_jump = (line.size - p.size).abs() > 1.5;
                let style_jump = line.bold != p.bold;
                let indent_more = line.left > p.left + 1.5 * line.size;
                let outdent = lines_in_para > 1 && line.left < p.left - 1.5 * line.size;
                gap > 0.6 * lh || gap < -lh || size_jump || style_jump || indent_more || outdent
            }
        };

        if start_new {
            paras.push(Para {
                text: line.text.clone(),
                size: line.size,
                bold: line.bold,
                top: line.top,
            });
            lines_in_para = 1;
        } else if let Some(para) = paras.last_mut() {
            join_line(&mut para.text, &line.text);
            lines_in_para += 1;
        }
        prev = Some(line);
    }
    paras
}

/// Menyambung baris ke paragraf; kata yang dipotong tanda hubung di akhir baris disatukan lagi.
fn join_line(para: &mut String, next: &str) {
    let ends_hyphen = para.ends_with('-')
        && para
            .chars()
            .rev()
            .nth(1)
            .map(|c| c.is_alphabetic())
            .unwrap_or(false);
    let next_lower = next.chars().next().map(|c| c.is_lowercase()).unwrap_or(false);
    if ends_hyphen && next_lower {
        para.pop();
    } else {
        para.push(' ');
    }
    para.push_str(next);
}

/// Ukuran huruf yang paling sering muncul (dibobot jumlah karakter), sebagai ukuran isi.
pub fn body_size(paras: &[Para]) -> f32 {
    use std::collections::HashMap;
    let mut count: HashMap<i32, usize> = HashMap::new();
    for p in paras {
        *count.entry((p.size * 2.0).round() as i32).or_default() += p.text.chars().count();
    }
    count
        .into_iter()
        .max_by_key(|(k, n)| (*n, -*k))
        .map(|(k, _)| k as f32 / 2.0)
        .unwrap_or(11.0)
}

/// Paragraf dianggap judul jika hurufnya jauh lebih besar dari ukuran isi.
pub fn is_heading(p: &Para, body: f32) -> bool {
    p.size >= body * 1.25 && p.text.chars().count() < 200
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frag(text: &str, left: f32, top: f32, size: f32) -> Frag {
        Frag {
            text: text.into(),
            left,
            right: left + text.len() as f32 * size * 0.5,
            top,
            bottom: top - size,
            size,
            bold: false,
        }
    }

    #[test]
    fn potongan_satu_baris_disatukan() {
        let lines = to_lines(vec![frag("dunia", 100.0, 700.0, 12.0), frag("Halo", 50.0, 700.5, 12.0)]);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "Halo dunia");
    }

    #[test]
    fn baris_rapat_satu_paragraf_jarak_lebar_paragraf_baru() {
        let lines = to_lines(vec![
            frag("Judul Besar", 50.0, 760.0, 20.0),
            frag("Baris pertama yang pan-", 50.0, 700.0, 12.0),
            frag("jang sekali.", 50.0, 686.0, 12.0),
            frag("Paragraf kedua.", 50.0, 640.0, 12.0),
        ]);
        let paras = to_paragraphs(&lines);
        let texts: Vec<&str> = paras.iter().map(|p| p.text.as_str()).collect();
        assert_eq!(texts, vec!["Judul Besar", "Baris pertama yang panjang sekali.", "Paragraf kedua."]);
        let body = body_size(&paras);
        assert_eq!(body, 12.0);
        assert!(is_heading(&paras[0], body));
        assert!(!is_heading(&paras[1], body));
    }

    #[test]
    fn indentasi_memulai_paragraf_baru() {
        let lines = to_lines(vec![
            frag("Baris satu.", 50.0, 700.0, 12.0),
            frag("Baris baru menjorok.", 80.0, 686.0, 12.0),
            frag("lanjutan.", 50.0, 672.0, 12.0),
        ]);
        let paras = to_paragraphs(&lines);
        assert_eq!(paras.len(), 2);
        assert_eq!(paras[1].text, "Baris baru menjorok. lanjutan.");
    }
}
