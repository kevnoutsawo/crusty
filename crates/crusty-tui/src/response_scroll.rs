//! Scroll math for the response body viewer.
//!
//! The response body uses [`ratatui::widgets::Paragraph`] with word wrapping,
//! so the visible row count is not simply the number of newlines: a long
//! line gets wrapped across multiple rows. The functions here approximate
//! the wrapped row count well enough to clamp the scroll offset and drive a
//! scrollbar without pulling in ratatui's unstable `line_count` API.

use unicode_width::UnicodeWidthStr;

/// Approximate the number of terminal rows a paragraph occupies after
/// word-wrapping to `width` columns. `width` of 0 collapses to the raw
/// newline count (one row per logical line).
///
/// This counts each `\n`-separated line and adds extra rows for any line
/// whose display width exceeds `width`. It is a conservative approximation —
/// it does not account for word-boundary wrapping behavior, so the actual
/// rendered height may be one row taller than reported when a word would
/// otherwise be split. For scroll clamping, a slight underestimate just
/// means the last row may remain visible at maximum scroll, which is the
/// desired behavior.
pub fn wrapped_row_count(text: &str, width: u16) -> u16 {
    if text.is_empty() {
        return 0;
    }

    let logical_lines = text.split('\n');
    let mut rows: usize = 0;

    if width == 0 {
        return logical_lines.count().min(u16::MAX as usize) as u16;
    }

    let w = width as usize;
    for line in logical_lines {
        let display = UnicodeWidthStr::width(line);
        rows = rows.saturating_add(if display == 0 { 1 } else { display.div_ceil(w) });
    }

    rows.min(u16::MAX as usize) as u16
}

/// The maximum valid scroll offset given content height and viewport height.
/// Returns 0 when the content fits entirely in the viewport.
pub fn max_scroll(content_rows: u16, viewport_rows: u16) -> u16 {
    content_rows.saturating_sub(viewport_rows)
}

/// Clamp a scroll offset to the valid range `[0, max_scroll]`.
pub fn clamp(scroll: u16, content_rows: u16, viewport_rows: u16) -> u16 {
    scroll.min(max_scroll(content_rows, viewport_rows))
}

/// Page size for `PageDown`/`PageUp`. Leaves one row of context overlap so
/// the user doesn't lose their place across paginated jumps.
pub fn page_size(viewport_rows: u16) -> u16 {
    viewport_rows.saturating_sub(1).max(1)
}

/// Half-page size for `Ctrl+D`/`Ctrl+U`. Always at least 1.
pub fn half_page_size(viewport_rows: u16) -> u16 {
    (viewport_rows / 2).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text_has_zero_rows() {
        assert_eq!(wrapped_row_count("", 80), 0);
    }

    #[test]
    fn single_short_line_is_one_row() {
        assert_eq!(wrapped_row_count("hello", 80), 1);
    }

    #[test]
    fn newlines_create_rows() {
        assert_eq!(wrapped_row_count("a\nb\nc", 80), 3);
    }

    #[test]
    fn trailing_newline_adds_blank_row() {
        // "a\n" splits into ["a", ""] -> 2 rows. This matches ratatui's
        // Paragraph rendering, which draws an empty line for the trailing \n.
        assert_eq!(wrapped_row_count("a\n", 80), 2);
    }

    #[test]
    fn long_line_wraps() {
        let text = "x".repeat(200);
        // 200 chars / 80 cols = 3 rows (ceil)
        assert_eq!(wrapped_row_count(&text, 80), 3);
    }

    #[test]
    fn wrap_counts_per_logical_line() {
        let line_a = "x".repeat(100); // 2 rows at width 80
        let line_b = "y".repeat(50); //  1 row
        let text = format!("{line_a}\n{line_b}");
        assert_eq!(wrapped_row_count(&text, 80), 3);
    }

    #[test]
    fn zero_width_falls_back_to_newline_count() {
        assert_eq!(wrapped_row_count("a\nb\nc", 0), 3);
    }

    #[test]
    fn unicode_width_counted() {
        // Each CJK character is 2 cols wide.
        let text = "漢".repeat(40); // 80 display cols -> 1 row
        assert_eq!(wrapped_row_count(&text, 80), 1);
        let text = "漢".repeat(41); // 82 cols -> 2 rows
        assert_eq!(wrapped_row_count(&text, 80), 2);
    }

    #[test]
    fn max_scroll_when_content_fits() {
        assert_eq!(max_scroll(5, 10), 0);
        assert_eq!(max_scroll(10, 10), 0);
    }

    #[test]
    fn max_scroll_exposes_only_overflow() {
        assert_eq!(max_scroll(100, 25), 75);
    }

    #[test]
    fn clamp_keeps_in_range() {
        assert_eq!(clamp(0, 100, 25), 0);
        assert_eq!(clamp(50, 100, 25), 50);
        assert_eq!(clamp(75, 100, 25), 75);
        assert_eq!(clamp(76, 100, 25), 75);
        assert_eq!(clamp(u16::MAX, 100, 25), 75);
    }

    #[test]
    fn clamp_collapses_to_zero_when_content_fits() {
        assert_eq!(clamp(42, 10, 25), 0);
    }

    #[test]
    fn page_size_overlaps_by_one_row() {
        assert_eq!(page_size(25), 24);
        assert_eq!(page_size(2), 1);
    }

    #[test]
    fn page_size_floor_one() {
        assert_eq!(page_size(1), 1);
        assert_eq!(page_size(0), 1);
    }

    #[test]
    fn half_page_size_floor_one() {
        assert_eq!(half_page_size(10), 5);
        assert_eq!(half_page_size(1), 1);
        assert_eq!(half_page_size(0), 1);
    }
}
