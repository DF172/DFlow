//! Shared helpers for fitting text into width-constrained widgets.
//!
//! iced has no built-in ellipsis, so long user content (file names, layer
//! names, paths, property values) would otherwise wrap and break row
//! heights or spill past fixed-width panels. `elide` trims to a character
//! budget and marks the cut with a single ellipsis.
//!
//! [`elided`] is the only spelling view code should use: it owns both halves
//! of the contract, the cut label and the hover tip that restores the whole
//! string, so a caller cannot ship one without the other.

use iced::Element;

/// Truncate `s` to at most `max` characters, replacing the overflow with a
/// trailing ellipsis. `max` includes the ellipsis character; `max == 0`
/// yields an empty string. Counts by `char` so multi-byte text is safe.
pub fn elide(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    if max == 0 {
        return String::new();
    }
    let mut out: String = s.chars().take(max - 1).collect();
    out.push('…');
    out
}

/// Whether [`elide`] would cut `s` at `max`.
pub fn is_cut(s: &str, max: usize) -> bool {
    s.chars().count() > max
}

/// The elided label, with the whole string on hover when the label was cut.
///
/// `label` builds the widget around the shown text, so each call site keeps its
/// own voice (size, style, width, alignment); this owns only the two decisions
/// that must not diverge: what is painted, and whether the tip is needed.
///
/// `theme` is passed straight to the tip: a glass pane resolves its colours when
/// it is built, so every renderer that can emit one has to be handed the palette
/// (the same rule `ribbon::widgets::RenderCtx::theme` records).
pub fn elided<'a, M: 'a>(
    s: &str,
    max: usize,
    theme: &iced::Theme,
    label: impl FnOnce(String) -> Element<'a, M>,
) -> Element<'a, M> {
    let shown = elide(s, max);
    let cut = is_cut(s, max);
    let element = label(shown);
    if cut {
        crate::ui::kit::tip::full(element, s.to_string(), theme)
    } else {
        element
    }
}

#[cfg(test)]
mod tests {
    use super::{elide, is_cut};

    #[test]
    fn short_unchanged() {
        assert_eq!(elide("abc", 8), "abc");
        assert_eq!(elide("abc", 3), "abc");
    }

    #[test]
    fn long_truncated() {
        assert_eq!(elide("abcdef", 4), "abc…");
        assert_eq!(elide("abcdef", 1), "…");
        assert_eq!(elide("abcdef", 0), "");
    }

    #[test]
    fn multibyte_safe() {
        // No panic on a char boundary and the budget counts chars.
        assert_eq!(elide("çağrışım", 4), "çağ…");
    }

    #[test]
    fn a_label_is_cut_only_when_it_overflows() {
        assert!(!is_cut("abc", 3), "an exact fit is not a cut");
        assert!(is_cut("abcd", 3));
        assert!(is_cut("çağrışım", 4), "the count is chars, not bytes");
        assert!(!is_cut("", 0));
    }

    fn walk(dir: &std::path::Path, visit: &mut impl FnMut(&str, &str)) {
        for entry in std::fs::read_dir(dir).expect("read_dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(&path, visit);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                let source = std::fs::read_to_string(&path).expect("read source");
                visit(&path.to_string_lossy().replace('\\', "/"), &source);
            }
        }
    }

    /// G9's whole-tree rule: view code does not elide on its own. Every site
    /// goes through [`elided`], the one spelling that also attaches the full
    /// text; a bare `elide` in a view would cut a name with no way back.
    ///
    /// The count floor is the second half of the gate: deleting the sink's call
    /// sites would otherwise satisfy "no bare elide" by showing nothing at all.
    #[test]
    fn no_view_elides_without_the_tip() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut bare = Vec::new();
        let mut wrapped = 0usize;
        for dir in ["src/ui", "src/app/view"] {
            walk(&root.join(dir), &mut |path, source| {
                if path.ends_with("src/ui/text_util.rs") {
                    return;
                }
                let paint = source.split("\n#[cfg(test)]").next().unwrap_or(source);
                if paint.contains("text_util::elide(") {
                    bare.push(path.to_string());
                }
                wrapped += paint.matches("text_util::elided(").count();
            });
        }
        assert!(
            bare.is_empty(),
            "these views cut text with no way to read it whole: {bare:?}"
        );
        assert!(
            wrapped >= 15,
            "the sink lost call sites: only {wrapped} labels go through `elided`"
        );
    }

    /// The sink owns both halves of the contract: the cut label and the tip
    /// that restores it, and the tip exists only when the label was cut.
    #[test]
    fn the_sink_pairs_the_cut_with_its_tip() {
        let source = include_str!("text_util.rs");
        let paint = source.split("\n#[cfg(test)]").next().unwrap_or(source);
        assert!(paint.contains("let cut = is_cut(s, max);"));
        assert!(paint.contains("crate::ui::kit::tip::full(element, s.to_string(), theme)"));
        assert!(
            paint.contains("if cut {"),
            "a tip on every label would pop on text that fits"
        );
    }
}
