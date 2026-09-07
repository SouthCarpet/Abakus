use std::collections::{BTreeMap, BTreeSet};
use ttf_parser::Face;
use unicode_normalization::UnicodeNormalization;

pub(crate) static REGULAR_BYTES: &[u8] =
    include_bytes!("../../assets/report-fonts/NotoSans-Regular.ttf");
pub(crate) static BOLD_BYTES: &[u8] = include_bytes!("../../assets/report-fonts/NotoSans-Bold.ttf");

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum FontKind {
    Regular,
    Bold,
}

pub(crate) struct PreparedText {
    pub text: String,
    pub replacements: u32,
}

pub(crate) fn face(kind: FontKind) -> Result<Face<'static>, String> {
    let bytes = match kind {
        FontKind::Regular => REGULAR_BYTES,
        FontKind::Bold => BOLD_BYTES,
    };
    Face::parse(bytes, 0).map_err(|_| "Zabudovaný font Noto Sans sa nedá načítať.".into())
}

pub(crate) fn prepare(input: &str, kind: FontKind) -> Result<PreparedText, String> {
    let font = face(kind)?;
    let normalized: String = input.nfc().collect();
    let mut text = String::new();
    let mut replacements = 0_u32;
    for scalar in normalized.chars() {
        if scalar == '\n' {
            text.push(scalar);
        } else if scalar == '\t' {
            text.push_str("    ");
        } else if scalar.is_control() || font.glyph_index(scalar).is_none() {
            text.push_str(&format!("[U+{:04X}]", u32::from(scalar)));
            replacements = replacements
                .checked_add(1)
                .ok_or_else(|| "Počet nahradených znakov pretiekol.".to_string())?;
        } else {
            text.push(scalar);
        }
    }
    Ok(PreparedText { text, replacements })
}

pub(crate) fn width(text: &str, kind: FontKind, size: f32) -> Result<f32, String> {
    let font = face(kind)?;
    let units = f32::from(font.units_per_em());
    let mut total = 0_f32;
    for scalar in text.chars().filter(|scalar| *scalar != '\n') {
        let glyph = font.glyph_index(scalar).ok_or_else(|| {
            format!(
                "Font neobsahuje vykresľovaný znak U+{:04X}.",
                u32::from(scalar)
            )
        })?;
        total += f32::from(font.glyph_hor_advance(glyph).unwrap_or(0)) * size / units;
    }
    Ok(total)
}

pub(crate) struct FontEncoding {
    pub kind: FontKind,
    pub by_scalar: BTreeMap<char, u16>,
}

impl FontEncoding {
    pub fn build(kind: FontKind, scalars: BTreeSet<char>) -> Result<Self, String> {
        if scalars.len() > usize::from(u16::MAX) {
            return Err("Report používa priveľa rôznych znakov pre PDF font.".into());
        }
        let by_scalar = scalars
            .into_iter()
            .enumerate()
            .map(|(index, scalar)| {
                let cid = u16::try_from(index + 1)
                    .map_err(|_| "Číslovanie znakov PDF fontu pretieklo.".to_string())?;
                Ok((scalar, cid))
            })
            .collect::<Result<_, String>>()?;
        Ok(Self { kind, by_scalar })
    }

    pub fn encode(&self, text: &str) -> Result<Vec<u8>, String> {
        let capacity = text
            .len()
            .checked_mul(2)
            .ok_or_else(|| "Veľkosť kódovaného textu PDF pretiekla.".to_string())?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(capacity)
            .map_err(|_| "Na kódovaný text PDF nie je dosť pamäte.".to_string())?;
        for scalar in text.chars() {
            let cid = self
                .by_scalar
                .get(&scalar)
                .ok_or_else(|| format!("Znak U+{:04X} nemá PDF kód.", u32::from(scalar)))?;
            bytes.extend_from_slice(&cid.to_be_bytes());
        }
        Ok(bytes)
    }
}
