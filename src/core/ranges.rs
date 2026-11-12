use std::ops::RangeInclusive;

use crate::error::AppError;

/// Mengubah teks seperti `1-3, 5, 8-10` menjadi daftar rentang halaman (mulai dari 1).
pub fn parse_ranges(text: &str, page_count: usize) -> Result<Vec<RangeInclusive<usize>>, AppError> {
    let bad = || AppError::BadRange(text.trim().to_string());
    let mut out = Vec::new();
    for part in text.split([',', ';']) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let normalized = part.replace(['–', '—'], "-");
        let (a, b) = match normalized.split_once('-') {
            Some((a, b)) => (num(a).ok_or_else(bad)?, num(b).ok_or_else(bad)?),
            None => {
                let n = num(&normalized).ok_or_else(bad)?;
                (n, n)
            }
        };
        if a == 0 || b == 0 || a > b || b > page_count {
            return Err(bad());
        }
        out.push(a..=b);
    }
    if out.is_empty() {
        return Err(bad());
    }
    Ok(out)
}

fn num(s: &str) -> Option<usize> {
    let s = s.trim();
    if s.is_empty() || !s.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// Daftar nomor halaman unik dan berurutan dari kumpulan rentang.
pub fn page_list(ranges: &[RangeInclusive<usize>]) -> Vec<usize> {
    let mut v: Vec<usize> = ranges.iter().flat_map(|r| r.clone()).collect();
    v.sort_unstable();
    v.dedup();
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rentang_campuran() {
        assert_eq!(parse_ranges("1-3, 5, 8-10", 10).unwrap(), vec![1..=3, 5..=5, 8..=10]);
    }

    #[test]
    fn spasi_dan_tanda_pisah_panjang() {
        assert_eq!(parse_ranges(" 2 – 4 ;7", 9).unwrap(), vec![2..=4, 7..=7]);
    }

    #[test]
    fn ditolak() {
        for t in ["3-1", "0", "abc", "1-11", "", " , ", "1--2", "-3", "2-"] {
            assert!(parse_ranges(t, 10).is_err(), "seharusnya ditolak: {t:?}");
        }
    }

    #[test]
    fn daftar_halaman_unik() {
        assert_eq!(page_list(&[3..=5, 1..=4]), vec![1, 2, 3, 4, 5]);
    }
}
