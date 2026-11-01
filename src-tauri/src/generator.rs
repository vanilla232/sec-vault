use rand::rngs::OsRng;
use rand::seq::SliceRandom;
use rand::Rng;
use serde::Deserialize;

const LOWER: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const UPPER: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &[u8] = b"0123456789";
const SYMBOLS: &[u8] = b"!@#$%^&*()-_=+[]{};:,.<>?/";
const AMBIGUOUS: &[u8] = b"0O1lI";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenOptions {
    pub length: u32,
    pub upper: bool,
    pub lower: bool,
    pub digits: bool,
    pub symbols: bool,
    #[serde(default)]
    pub exclude_ambiguous: bool,
}

impl Default for GenOptions {
    fn default() -> Self {
        Self {
            length: 20,
            upper: true,
            lower: true,
            digits: true,
            symbols: true,
            exclude_ambiguous: true,
        }
    }
}

pub fn generate(opts: &GenOptions) -> Result<String, String> {
    let classes = [
        (opts.lower, LOWER),
        (opts.upper, UPPER),
        (opts.digits, DIGITS),
        (opts.symbols, SYMBOLS),
    ];

    let enabled: Vec<Vec<u8>> = classes
        .iter()
        .filter(|(on, _)| *on)
        .map(|(_, set)| {
            set.iter()
                .copied()
                .filter(|c| !opts.exclude_ambiguous || !AMBIGUOUS.contains(c))
                .collect()
        })
        .collect();

    if enabled.len() < 2 {
        return Err("at least two character classes must be enabled".into());
    }
    if (opts.length as usize) < enabled.len() {
        return Err(format!(
            "length must be at least {} to include every selected class",
            enabled.len()
        ));
    }
    if opts.length > 256 {
        return Err("length is too large".into());
    }

    let mut rng = OsRng;
    let mut chars: Vec<u8> = Vec::with_capacity(opts.length as usize);

    for set in &enabled {
        if set.is_empty() {
            return Err("a selected character class is empty after filtering".into());
        }
        let idx = rng.gen_range(0..set.len());
        chars.push(set[idx]);
    }

    let all: Vec<u8> = enabled.iter().flatten().copied().collect();
    while chars.len() < opts.length as usize {
        let idx = rng.gen_range(0..all.len());
        chars.push(all[idx]);
    }

    chars.shuffle(&mut rng);

    String::from_utf8(chars).map_err(|_| "internal: non-utf8 symbol".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_correct_length() {
        let opts = GenOptions { length: 24, ..Default::default() };
        let pw = generate(&opts).unwrap();
        assert_eq!(pw.len(), 24);
    }

    #[test]
    fn includes_every_selected_class() {
        for _ in 0..20 {
            let pw = generate(&GenOptions::default()).unwrap();
            assert!(pw.bytes().any(|c| c.is_ascii_lowercase()));
            assert!(pw.bytes().any(|c| c.is_ascii_uppercase()));
            assert!(pw.bytes().any(|c| c.is_ascii_digit()));
            assert!(pw.bytes().any(|c| SYMBOLS.contains(&c)));
        }
    }

    #[test]
    fn honors_excluded_classes() {
        let opts = GenOptions {
            length: 40,
            symbols: false,
            ..Default::default()
        };
        for _ in 0..20 {
            let pw = generate(&opts).unwrap();
            assert!(!pw.bytes().any(|c| SYMBOLS.contains(&c)));
        }
    }

    #[test]
    fn excludes_ambiguous_when_requested() {
        let opts = GenOptions {
            length: 100,
            symbols: false,
            exclude_ambiguous: true,
            ..Default::default()
        };
        for _ in 0..50 {
            let pw = generate(&opts).unwrap();
            assert!(!pw.bytes().any(|c| AMBIGUOUS.contains(&c)));
        }
    }

    #[test]
    fn rejects_single_class() {
        let opts = GenOptions {
            length: 20,
            lower: true,
            upper: false,
            digits: false,
            symbols: false,
            exclude_ambiguous: false,
        };
        assert!(generate(&opts).is_err());
    }

    #[test]
    fn rejects_too_short_for_classes() {
        let opts = GenOptions { length: 2, ..Default::default() };
        assert!(generate(&opts).is_err());
    }

    #[test]
    fn passwords_are_not_identical() {
        let opts = GenOptions::default();
        assert_ne!(generate(&opts).unwrap(), generate(&opts).unwrap());
    }
}