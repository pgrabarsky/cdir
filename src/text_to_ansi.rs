use std::io::{self, Write as IoWrite};

use crossterm::{
    queue,
    style::{Attribute, Color as CrosstermColor, ContentStyle, Stylize},
};
use ratatui::{
    style::{Color, Modifier, Style},
    text::Text,
};

/// Converts ratatui Text to ANSI-colored string using Crossterm
pub fn text_to_ansi(text: &Text) -> String {
    let mut result = Vec::new(); // Use a Vec<u8> for efficiency with queue!
    let mut stdout = io::Cursor::new(&mut result); // Fake writer for queueing commands
    let mut current_style: Option<Style> = None;

    let mut first_line = true;

    for line in &text.lines {
        if !first_line {
            let _ = writeln!(&mut stdout);
        }
        for span in &line.spans {
            let effective_style = line.style.patch(span.style);

            if current_style != Some(effective_style) {
                // Reset if needed
                if current_style.is_some() {
                    let _ = queue!(&mut stdout, crossterm::style::ResetColor);
                    let _ = queue!(
                        &mut stdout,
                        crossterm::style::SetAttribute(Attribute::Reset)
                    );
                }
                // Apply new style via Crossterm
                let ct_style = to_crossterm_style(&effective_style);
                let _ = queue!(&mut stdout, crossterm::style::SetStyle(ct_style));
            }

            let _ = write!(&mut stdout, "{}", span.content);

            current_style = Some(effective_style);
        }

        // End of line: reset and newline
        let _ = queue!(&mut stdout, crossterm::style::ResetColor);
        let _ = queue!(
            &mut stdout,
            crossterm::style::SetAttribute(Attribute::Reset)
        );
        first_line = false;
        current_style = None;
    }

    // Final reset if needed
    if current_style.is_some() {
        let _ = queue!(&mut stdout, crossterm::style::ResetColor);
        let _ = queue!(
            &mut stdout,
            crossterm::style::SetAttribute(Attribute::Reset)
        );
    }

    // Flush to get the string
    String::from_utf8_lossy(&result).to_string()
}

/// Convert Ratatui Style to Crossterm ContentStyle
fn to_crossterm_style(style: &Style) -> ContentStyle {
    let mut ct_style = ContentStyle::new();

    // Colors
    if let Some(fg) = style.fg {
        ct_style = ct_style.with(to_crossterm_color(fg));
    }
    if let Some(bg) = style.bg {
        ct_style = ct_style.on(to_crossterm_color(bg));
    }

    // Add modifiers
    if style.add_modifier.contains(Modifier::BOLD) {
        ct_style = ct_style.attribute(Attribute::Bold);
    }
    if style.add_modifier.contains(Modifier::DIM) {
        ct_style = ct_style.attribute(Attribute::Dim);
    }
    if style.add_modifier.contains(Modifier::ITALIC) {
        ct_style = ct_style.attribute(Attribute::Italic);
    }
    if style.add_modifier.contains(Modifier::UNDERLINED) {
        ct_style = ct_style.attribute(Attribute::Underlined);
    }
    if style.add_modifier.contains(Modifier::SLOW_BLINK) {
        ct_style = ct_style.attribute(Attribute::SlowBlink);
    }
    if style.add_modifier.contains(Modifier::RAPID_BLINK) {
        ct_style = ct_style.attribute(Attribute::RapidBlink);
    }
    if style.add_modifier.contains(Modifier::REVERSED) {
        ct_style = ct_style.attribute(Attribute::Reverse);
    }
    if style.add_modifier.contains(Modifier::HIDDEN) {
        ct_style = ct_style.attribute(Attribute::Hidden);
    }
    if style.add_modifier.contains(Modifier::CROSSED_OUT) {
        ct_style = ct_style.attribute(Attribute::CrossedOut);
    }

    // Remove modifiers (Crossterm uses specific resets)
    if style.sub_modifier.contains(Modifier::BOLD) {
        ct_style = ct_style.attribute(Attribute::NormalIntensity); // Resets bold/dim
    }
    if style.sub_modifier.contains(Modifier::DIM) {
        ct_style = ct_style.attribute(Attribute::NormalIntensity);
    }
    if style.sub_modifier.contains(Modifier::ITALIC) {
        ct_style = ct_style.attribute(Attribute::NoItalic);
    }
    if style.sub_modifier.contains(Modifier::UNDERLINED) {
        ct_style = ct_style.attribute(Attribute::NoUnderline);
    }
    if style
        .sub_modifier
        .contains(Modifier::SLOW_BLINK | Modifier::RAPID_BLINK)
    {
        ct_style = ct_style.attribute(Attribute::NoBlink);
    }
    if style.sub_modifier.contains(Modifier::REVERSED) {
        ct_style = ct_style.attribute(Attribute::NoReverse);
    }
    if style.sub_modifier.contains(Modifier::HIDDEN) {
        ct_style = ct_style.attribute(Attribute::NoHidden);
    }
    if style.sub_modifier.contains(Modifier::CROSSED_OUT) {
        ct_style = ct_style.attribute(Attribute::NotCrossedOut);
    }

    ct_style
}

fn to_crossterm_color(color: Color) -> CrosstermColor {
    match color {
        Color::Reset => CrosstermColor::Reset,
        Color::Black => CrosstermColor::Black,
        Color::Red => CrosstermColor::DarkRed,
        Color::Green => CrosstermColor::DarkGreen,
        Color::Yellow => CrosstermColor::DarkYellow,
        Color::Blue => CrosstermColor::DarkBlue,
        Color::Magenta => CrosstermColor::DarkMagenta,
        Color::Cyan => CrosstermColor::DarkCyan,
        Color::Gray => CrosstermColor::DarkGrey,
        Color::DarkGray => CrosstermColor::DarkGrey,
        Color::LightRed => CrosstermColor::Red,
        Color::LightGreen => CrosstermColor::Green,
        Color::LightYellow => CrosstermColor::Yellow,
        Color::LightBlue => CrosstermColor::Blue,
        Color::LightMagenta => CrosstermColor::Magenta,
        Color::LightCyan => CrosstermColor::Cyan,
        Color::White => CrosstermColor::White,
        Color::Indexed(i) => CrosstermColor::AnsiValue(i),
        Color::Rgb(r, g, b) => CrosstermColor::Rgb { r, g, b },
    }
}
