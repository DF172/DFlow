//! Script classification for drawing text that needs shaping (#141).
//!
//! Non-LFF scripts are outlined from the user's installed system fonts (see
//! `ttf_glyph::build_fallback`). Runs in a script that needs shaping — Arabic,
//! Hebrew, Indic, Thai, Lao — are laid out by the shaper instead, using
//! [`fallback_family`] when the style's own face is a stroke font.

/// Family the shaper uses for a run whose style has no TrueType face.
pub fn fallback_family() -> &'static str {
    "Noto Sans"
}

pub fn requires_shaping(text: &str) -> bool {
    text.chars().any(|ch| {
        matches!(
            ch as u32,
            0x0590..=0x05FF
                | 0x0600..=0x06FF
                | 0x0750..=0x077F
                | 0x08A0..=0x08FF
                | 0x0900..=0x097F
                | 0x0E00..=0x0E7F
                | 0x0E80..=0x0EFF
                | 0xA8E0..=0xA8FF
                | 0xFB1D..=0xFB4F
                | 0xFB50..=0xFDFF
                | 0xFE70..=0xFEFF
        )
    })
}

/// Font-set generation for cache keys. The installed font set is fixed for
/// the life of the process, so this never changes.
pub fn generation() -> u64 {
    0
}