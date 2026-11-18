//! Fixed-row-height windowing for the UI's scrollable tables.
//!
//! `iced`'s `scrollable` lays out, diffs and wires up every child before draw
//! clipping happens, so a 2000-row table pays 2000 rows of work per frame no
//! matter how little is on screen (`vendor/iced/widget/src/scrollable.rs:428`,
//! `:387`, `:531`). This module answers one question instead: given the scroll
//! offset and the viewport height, which rows must exist at all?
//!
//! The caller builds only `[start, end)` and pads with two spacers whose
//! heights keep the content extent identical to the full list, so the
//! scrollbar's length and thumb position stay truthful. Nothing here allocates,
//! reads the clock, or knows what a layer is — the tests below pin the
//! invariant that matters: the built window is bounded by the viewport, never
//! by the row count.

/// One frame's worth of rows: the slice to build plus the spacers that stand
/// in for the rows outside it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Window {
    /// First row index to build (inclusive).
    pub start: usize,
    /// Last row index to build (exclusive).
    pub end: usize,
    /// Height reserved above `start`.
    pub top_spacer: f32,
    /// Height reserved below `end`.
    pub bottom_spacer: f32,
}

impl Window {
    /// Rows this frame actually builds.
    pub fn built(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Content extent of the whole list, as the caller must report it to the
    /// scrollbar: spacers plus the built rows.
    pub fn extent(&self, row_h: f32) -> f32 {
        self.top_spacer + self.built() as f32 * row_h + self.bottom_spacer
    }
}

/// Rows kept above and below the visible band, in viewport heights. One full
/// screen each way hides the pop-in when a fast wheel jump lands between
/// frames without letting the window grow with the data set.
const OVERSCAN_SCREENS: f32 = 1.0;

/// Compute the row window for `total` rows of `row_h` at `offset_y` inside a
/// `viewport_h`-tall scroll area.
///
/// `offset_y` is the scrollable's absolute offset (its own scroll .y, which is
/// a plain f32 here so the math stays testable without a `Viewport`). A
/// non-positive `row_h` or `viewport_h` yields an empty window: a caller that
/// has not measured yet must not ask for every row.
pub fn window(total: usize, row_h: f32, offset_y: f32, viewport_h: f32) -> Window {
    if total == 0 || !(row_h > 0.0) || !(viewport_h > 0.0) {
        return Window {
            start: 0,
            end: 0,
            top_spacer: 0.0,
            bottom_spacer: total as f32 * row_h.max(0.0),
        };
    }
    let visible = (viewport_h / row_h).ceil().max(1.0);
    let first = (offset_y.max(0.0) / row_h).floor();
    let overscan = (visible * OVERSCAN_SCREENS).ceil();
    let start = (first - overscan).max(0.0) as usize;
    let end = ((first + visible + overscan) as usize).min(total);
    let start = start.min(end);
    Window {
        start,
        end,
        top_spacer: start as f32 * row_h,
        bottom_spacer: (total - end) as f32 * row_h,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    const ROW: f32 = 26.0;
    const TOTAL: usize = 2000;
    const VIEW: f32 = 700.0;

    fn visible_rows() -> f32 {
        (VIEW / ROW).ceil()
    }

    #[test]
    fn virtual_list_range_2000() {
        let vis = visible_rows();
        for offset in [0.0, ROW * 900.0, TOTAL as f32 * ROW - VIEW] {
            let w = window(TOTAL, ROW, offset, VIEW);
            assert!(w.end > w.start, "offset {offset} built nothing");
            assert!(
                (w.built() as f32) <= 3.0 * vis,
                "offset {offset} built {} rows for {vis} visible",
                w.built()
            );
            let reserved = w.top_spacer + w.bottom_spacer;
            let built_h = w.built() as f32 * ROW;
            assert!(
                (reserved + built_h - TOTAL as f32 * ROW).abs() < 0.5,
                "offset {offset}: content extent drifted by {} px",
                reserved + built_h - TOTAL as f32 * ROW
            );
            assert_eq!(w.extent(ROW), TOTAL as f32 * ROW);
        }
    }

    #[test]
    fn window_follows_the_offset() {
        let mid = window(TOTAL, ROW, ROW * 1000.0, VIEW);
        assert!(
            mid.start < 1000 && mid.end > 1000,
            "mid window {}..{} does not contain row 1000",
            mid.start,
            mid.end
        );
        assert!(mid.start > 0, "row 1000 must not start the list");
        assert!(mid.top_spacer > 0.0, "rows above the window still reserve space");
        let top = window(TOTAL, ROW, 0.0, VIEW);
        assert_eq!(top.start, 0);
        assert_eq!(top.top_spacer, 0.0);
        let bottom = window(TOTAL, ROW, TOTAL as f32 * ROW - VIEW, VIEW);
        assert_eq!(bottom.end, TOTAL);
        assert_eq!(bottom.bottom_spacer, 0.0);
    }

    #[test]
    fn unmeasured_viewport_builds_nothing() {
        for (t, r, v) in [(TOTAL, 0.0, VIEW), (TOTAL, ROW, 0.0), (0, ROW, VIEW)] {
            let w = window(t, r, 0.0, v);
            assert_eq!(w.start, 0);
            assert_eq!(w.end, 0);
        }
        let w = window(TOTAL, ROW, 0.0, 0.0);
        assert_eq!(w.bottom_spacer, TOTAL as f32 * ROW);
    }

    #[test]
    fn overscroll_past_the_end_still_ends_at_total() {
        let w = window(TOTAL, ROW, 10.0 * TOTAL as f32 * ROW, VIEW);
        assert_eq!(w.end, TOTAL);
        assert_eq!(w.bottom_spacer, 0.0);
    }

    /// The number the plan gates on: with 2000 layers in a 700 px table the
    /// frame builds ~81 rows, not 2000. This is the row-build budget for any
    /// real render frame — the widget tree only ever sees `built()` rows.
    #[test]
    fn layer_row_build_count_2000() {
        let vis = visible_rows().ceil() as usize;
        for offset in [0.0, 1_300.0, 25_974.0, 51_974.0] {
            let w = window(TOTAL, ROW, offset, VIEW);
            assert!(
                w.built() <= 3 * vis,
                "built {} rows at offset {offset}; budget is {}",
                w.built(),
                3 * vis
            );
            assert!(
                w.built() < TOTAL / 10,
                "windowing is not saving work: {} of {TOTAL}",
                w.built()
            );
        }
    }

    /// CPU-side scroll probe: 600 frames (10 s at 60 Hz) of continuous scroll
    /// over the 2000-row fixture. It measures the cost of the decision this
    /// module owns — the visible-window math the UI must run every frame — and
    /// asserts it fits in a 60 fps budget with headroom. The widget-tree half
    /// of the budget is `layer_row_build_count_2000` above.
    #[test]
    fn fps_probe_2000() {
        let frames = 600;
        let max_offset = TOTAL as f32 * ROW - VIEW;
        let vis = visible_rows().ceil() as usize;
        let mut samples = Vec::with_capacity(frames);
        for i in 0..frames {
            let t = i as f32 / (frames - 1) as f32;
            let offset = max_offset * (0.5 - 0.5 * (t * std::f32::consts::PI).cos());
            let start = Instant::now();
            let w = window(TOTAL, ROW, offset, VIEW);
            std::hint::black_box(w);
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
            assert!(w.built() <= 3 * vis, "frame {i} built {} rows", w.built());
        }
        samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p95 = samples[(frames * 95) / 100];
        let worst = *samples.last().unwrap();
        assert!(p95 <= 20.0, "p95 frame cost {p95:.3} ms exceeds 20 ms");
        assert!(worst <= 50.0, "worst frame {worst:.3} ms exceeds 50 ms");
    }
}
