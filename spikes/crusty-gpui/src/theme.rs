//! Crusty's palette (the same design-system colors the TUI uses).

use gpui::{Global, Hsla, rgb, rgba};

#[derive(Clone, Copy)]
pub struct Theme {
    pub bg: Hsla,
    pub surface: Hsla,
    pub elevated: Hsla,
    pub border: Hsla,
    pub text: Hsla,
    pub text_muted: Hsla,
    pub text_faint: Hsla,
    pub accent: Hsla,
    pub on_accent: Hsla,
    pub selection: Hsla,
    pub success: Hsla,
    pub warning: Hsla,
    pub danger: Hsla,
}

impl Global for Theme {}

pub fn dark() -> Theme {
    Theme {
        bg: rgb(0x0d1117).into(),
        surface: rgb(0x161b22).into(),
        elevated: rgb(0x21262d).into(),
        border: rgb(0x30363d).into(),
        text: rgb(0xe6edf3).into(),
        text_muted: rgb(0x8b949e).into(),
        text_faint: rgb(0x6e7681).into(),
        accent: rgb(0x58a6ff).into(),
        on_accent: rgb(0x0d1117).into(),
        selection: rgba(0x58a6ff55).into(),
        success: rgb(0x3fb950).into(),
        warning: rgb(0xd29922).into(),
        danger: rgb(0xf85149).into(),
    }
}
