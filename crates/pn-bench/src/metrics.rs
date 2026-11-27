//! Accuracy metrics: character/word error rate, diacritic accuracy, math exact match.

use unicode_normalization::UnicodeNormalization;

/// NFC-normalise and collapse all whitespace runs to single spaces.
pub fn normalize(s: &str) -> String {
    let nfc: String = s.nfc().collect();
    nfc.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Levenshtein distance between two token sequences.
pub fn edit_distance<T: PartialEq>(a: &[T], b: &[T]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for (i, x) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, y) in b.iter().enumerate() {
            let sub = prev[j] + usize::from(x != y);
            cur[j + 1] = sub.min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

/// Character error rate: edits / reference length (0.0 = perfect).
pub fn cer(reference: &str, hypothesis: &str) -> f64 {
    let r: Vec<char> = normalize(reference).chars().collect();
    let h: Vec<char> = normalize(hypothesis).chars().collect();
    if r.is_empty() {
        return if h.is_empty() { 0.0 } else { 1.0 };
    }
    edit_distance(&r, &h) as f64 / r.len() as f64
}

/// Word error rate: word edits / reference word count.
pub fn wer(reference: &str, hypothesis: &str) -> f64 {
    let r = normalize(reference);
    let h = normalize(hypothesis);
    let r: Vec<&str> = r.split(' ').filter(|w| !w.is_empty()).collect();
    let h: Vec<&str> = h.split(' ').filter(|w| !w.is_empty()).collect();
    if r.is_empty() {
        return if h.is_empty() { 0.0 } else { 1.0 };
    }
    edit_distance(&r, &h) as f64 / r.len() as f64
}

/// Letters we track separately because recognisers often get them wrong.
pub const DIACRITICS: &str = "æøåÆØÅäöüÄÖÜßéèêëÉÈÊáàâÁÀÂíìîÍÌÎóòôÓÒÔúùûÚÙÛçÇñÑğĞşŞıİ";

/// Fraction of diacritic letters in the reference that appear, in order, in the
/// hypothesis (longest common subsequence over diacritic letters only).
/// `None` if the reference has no diacritics.
pub fn diacritic_recall(reference: &str, hypothesis: &str) -> Option<f64> {
    let pick = |s: &str| {
        normalize(s)
            .chars()
            .filter(|c| DIACRITICS.contains(*c))
            .collect::<Vec<_>>()
    };
    let r = pick(reference);
    if r.is_empty() {
        return None;
    }
    let h = pick(hypothesis);
    let mut dp = vec![vec![0usize; h.len() + 1]; r.len() + 1];
    for i in 0..r.len() {
        for j in 0..h.len() {
            dp[i + 1][j + 1] = if r[i] == h[j] {
                dp[i][j] + 1
            } else {
                dp[i][j + 1].max(dp[i + 1][j])
            };
        }
    }
    Some(dp[r.len()][h.len()] as f64 / r.len() as f64)
}

/// Canonical form of a LaTeX expression for exact-match comparison.
pub fn normalize_latex(s: &str) -> String {
    let mut t = s.trim().trim_matches('$').to_string();
    for (from, to) in [
        ("\\left", ""),
        ("\\right", ""),
        ("\\displaystyle", ""),
        ("\\,", ""),
        (r"\;", ""),
        ("\\!", ""),
        ("\\cdot", "*"),
        ("\\times", "*"),
        ("\\dfrac", "\\frac"),
        ("\\tfrac", "\\frac"),
    ] {
        t = t.replace(from, to);
    }
    t.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Expression-level exact match after normalisation.
pub fn latex_exact(reference: &str, hypothesis: &str) -> bool {
    normalize_latex(reference) == normalize_latex(hypothesis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cer_and_wer() {
        assert_eq!(cer("hei på deg", "hei på deg"), 0.0);
        assert!((cer("abcd", "abed") - 0.25).abs() < 1e-9);
        assert!((wer("lekse til fredag", "lekse til torsdag") - 1.0 / 3.0).abs() < 1e-9);
        // whitespace differences don't count
        assert_eq!(cer("a  b\nc", "a b c"), 0.0);
        assert_eq!(cer("", ""), 0.0);
        assert_eq!(cer("", "x"), 1.0);
    }

    #[test]
    fn nfc_makes_composed_and_decomposed_equal() {
        // "å" as one code point vs "a" + combining ring
        assert_eq!(cer("på", "pa\u{030A}"), 0.0);
    }

    #[test]
    fn diacritics() {
        assert_eq!(diacritic_recall("abc", "abc"), None);
        assert_eq!(
            diacritic_recall("blåbærsyltetøy", "blåbærsyltetøy"),
            Some(1.0)
        );
        let r = diacritic_recall("blåbærsyltetøy", "blabærsyltetoy").unwrap();
        assert!((r - 1.0 / 3.0).abs() < 1e-9, "{r}");
    }

    #[test]
    fn latex_normalisation() {
        assert!(latex_exact(r"v = 12 + 3 \cdot 5", "v=12+3*5"));
        assert!(latex_exact(r"$\left(\frac{1}{2}\right)$", r"(\frac{1}{2})"));
        assert!(latex_exact(r"\dfrac{a}{b}", r"\frac{a}{b}"));
        assert!(!latex_exact("x^2", "x^3"));
    }
}
