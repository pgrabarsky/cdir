use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};

const DEFAULT_TITLE: fn() -> Option<String> = || Some(String::from("#1d5cba"));
const DEFAULT_BACKGROUND_COLOR: fn() -> Option<String> = || Some(String::from("#ffffff"));
const DEFAULT_LEFT_BACKGROUND_COLOR: fn() -> Option<String> = || None;
const DEFAULT_BORDER_COLOR: fn() -> Option<String> = || Some(String::from("#cccccc"));
const DEFAULT_TEXT: fn() -> Option<String> = || Some(String::from("#2a2a2a"));
const DEFAULT_TEXT_EM: fn() -> Option<String> = || Some(String::from("#009dc8"));
const DEFAULT_COLOR_DATE: fn() -> Option<String> = || Some(String::from("#888888"));
const DEFAULT_COLOR_PATH: fn() -> Option<String> = || Some(String::from("#2a2929"));
const DEFAULT_COLOR_HIGHLIGHT: fn() -> Option<String> = || Some(String::from("#fbe9a4"));
const DEFAULT_COLOR_SHORTCUT_NAME: fn() -> Option<String> = || Some(String::from("#00aa00"));

const DEFAULT_COLOR_FG_HEADER: fn() -> Option<String> = || Some(String::from("#ffffff"));
const DEFAULT_COLOR_BG_HEADER: fn() -> Option<String> = || Some(String::from("#2741b7"));

const DEFAULT_COLOR_DESCRIPTION: fn() -> Option<String> = || Some(String::from("#808080"));

const DEFAULT_FREE_TEXT_AREA_BG: fn() -> Option<String> = || Some(String::from("#f2f2f2"));

const DEFAULT_HOME_TILD: fn() -> Option<String> = || Some(String::from("#888888"));

const DEFAULT_NONE: fn() -> Option<String> = || None;
const DEFAULT_BOOL_NONE: fn() -> Option<bool> = || None;

/// Represents the color configuration for various UI elements.
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Theme {
    #[serde(default = "DEFAULT_NONE")]
    pub title: Option<String>,

    #[serde(default = "DEFAULT_BOOL_NONE")]
    pub title_bold: Option<bool>,

    #[serde(default = "DEFAULT_BOOL_NONE")]
    pub title_italic: Option<bool>,

    #[serde(default = "DEFAULT_NONE")]
    pub background: Option<String>,

    #[serde(default = "DEFAULT_NONE")]
    pub left_background: Option<String>,

    #[serde(default = "DEFAULT_NONE")]
    pub border: Option<String>,

    #[serde(default = "DEFAULT_NONE")]
    pub text: Option<String>,

    #[serde(default = "DEFAULT_BOOL_NONE")]
    pub text_bold: Option<bool>,

    #[serde(default = "DEFAULT_BOOL_NONE")]
    pub text_italic: Option<bool>,

    #[serde(default = "DEFAULT_NONE")]
    pub text_em: Option<String>,

    #[serde(default = "DEFAULT_BOOL_NONE")]
    pub text_em_bold: Option<bool>,

    #[serde(default = "DEFAULT_BOOL_NONE")]
    pub text_em_italic: Option<bool>,

    #[serde(default = "DEFAULT_NONE")]
    pub date: Option<String>,

    #[serde(default = "DEFAULT_BOOL_NONE")]
    pub date_bold: Option<bool>,

    #[serde(default = "DEFAULT_BOOL_NONE")]
    pub date_italic: Option<bool>,

    #[serde(default = "DEFAULT_NONE")]
    pub path: Option<String>,

    #[serde(default = "DEFAULT_BOOL_NONE")]
    pub path_bold: Option<bool>,

    #[serde(default = "DEFAULT_BOOL_NONE")]
    pub path_italic: Option<bool>,

    #[serde(default = "DEFAULT_NONE")]
    pub highlight: Option<String>,

    #[serde(default = "DEFAULT_NONE")]
    pub shortcut_name: Option<String>,

    #[serde(default = "DEFAULT_BOOL_NONE")]
    pub shortcut_name_bold: Option<bool>,

    #[serde(default = "DEFAULT_BOOL_NONE")]
    pub shortcut_name_italic: Option<bool>,

    #[serde(default = "DEFAULT_NONE")]
    pub header_fg: Option<String>,

    #[serde(default = "DEFAULT_NONE")]
    pub header_bg: Option<String>,

    #[serde(default = "DEFAULT_NONE")]
    pub description: Option<String>,

    #[serde(default = "DEFAULT_BOOL_NONE")]
    pub description_bold: Option<bool>,

    #[serde(default = "DEFAULT_BOOL_NONE")]
    pub description_italic: Option<bool>,

    #[serde(default = "DEFAULT_NONE")]
    pub free_text_area_bg: Option<String>,

    #[serde(default = "DEFAULT_NONE")]
    pub home_tilde: Option<String>,

    #[serde(default = "DEFAULT_BOOL_NONE")]
    pub home_tilde_bold: Option<bool>,

    #[serde(default = "DEFAULT_BOOL_NONE")]
    pub home_tilde_italic: Option<bool>,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            title: DEFAULT_TITLE(),
            title_bold: DEFAULT_BOOL_NONE(),
            title_italic: DEFAULT_BOOL_NONE(),
            background: DEFAULT_BACKGROUND_COLOR(),
            left_background: DEFAULT_LEFT_BACKGROUND_COLOR(),
            border: DEFAULT_BORDER_COLOR(),
            text: DEFAULT_TEXT(),
            text_bold: DEFAULT_BOOL_NONE(),
            text_italic: DEFAULT_BOOL_NONE(),
            text_em: DEFAULT_TEXT_EM(),
            text_em_bold: DEFAULT_BOOL_NONE(),
            text_em_italic: DEFAULT_BOOL_NONE(),
            date: DEFAULT_COLOR_DATE(),
            date_bold: DEFAULT_BOOL_NONE(),
            date_italic: DEFAULT_BOOL_NONE(),
            path: DEFAULT_COLOR_PATH(),
            path_bold: DEFAULT_BOOL_NONE(),
            path_italic: DEFAULT_BOOL_NONE(),
            highlight: DEFAULT_COLOR_HIGHLIGHT(),
            shortcut_name: DEFAULT_COLOR_SHORTCUT_NAME(),
            shortcut_name_bold: DEFAULT_BOOL_NONE(),
            shortcut_name_italic: DEFAULT_BOOL_NONE(),
            header_fg: DEFAULT_COLOR_FG_HEADER(),
            header_bg: DEFAULT_COLOR_BG_HEADER(),
            description: DEFAULT_COLOR_DESCRIPTION(),
            description_bold: DEFAULT_BOOL_NONE(),
            description_italic: DEFAULT_BOOL_NONE(),
            free_text_area_bg: DEFAULT_FREE_TEXT_AREA_BG(),
            home_tilde: DEFAULT_HOME_TILD(),
            home_tilde_bold: DEFAULT_BOOL_NONE(),
            home_tilde_italic: DEFAULT_BOOL_NONE(),
        }
    }
}

impl Theme {
    /// Returns a new Colors from self, or from colors when the Option is not set, or from default colors
    pub fn merge(&self, theme: &Theme) -> Theme {
        Theme {
            title: self
                .title
                .clone()
                .or(theme.title.clone())
                .or(DEFAULT_TITLE()),
            title_bold: self.title_bold.or(theme.title_bold).or(DEFAULT_BOOL_NONE()),
            title_italic: self
                .title_italic
                .or(theme.title_italic)
                .or(DEFAULT_BOOL_NONE()),
            background: self
                .background
                .clone()
                .or(theme.background.clone())
                .or(DEFAULT_BACKGROUND_COLOR()),
            left_background: self
                .left_background
                .clone()
                .or(theme.left_background.clone())
                .or(DEFAULT_LEFT_BACKGROUND_COLOR()),
            border: self
                .border
                .clone()
                .or(theme.border.clone())
                .or(DEFAULT_BORDER_COLOR()),
            text: self.text.clone().or(theme.text.clone()).or(DEFAULT_TEXT()),
            text_bold: self.text_bold.or(theme.text_bold).or(DEFAULT_BOOL_NONE()),
            text_italic: self
                .text_italic
                .or(theme.text_italic)
                .or(DEFAULT_BOOL_NONE()),
            text_em: self
                .text_em
                .clone()
                .or(theme.text_em.clone())
                .or(DEFAULT_TEXT_EM()),
            text_em_bold: self
                .text_em_bold
                .or(theme.text_em_bold)
                .or(DEFAULT_BOOL_NONE()),
            text_em_italic: self
                .text_em_italic
                .or(theme.text_em_italic)
                .or(DEFAULT_BOOL_NONE()),
            date: self
                .date
                .clone()
                .or(theme.date.clone())
                .or(DEFAULT_COLOR_DATE()),
            date_bold: self.date_bold.or(theme.date_bold).or(DEFAULT_BOOL_NONE()),
            date_italic: self
                .date_italic
                .or(theme.date_italic)
                .or(DEFAULT_BOOL_NONE()),
            path: self
                .path
                .clone()
                .or(theme.path.clone())
                .or(DEFAULT_COLOR_PATH()),
            path_bold: self.path_bold.or(theme.path_bold).or(DEFAULT_BOOL_NONE()),
            path_italic: self
                .path_italic
                .or(theme.path_italic)
                .or(DEFAULT_BOOL_NONE()),
            highlight: self
                .highlight
                .clone()
                .or(theme.highlight.clone())
                .or(DEFAULT_COLOR_HIGHLIGHT()),
            shortcut_name: self
                .shortcut_name
                .clone()
                .or(theme.shortcut_name.clone())
                .or(DEFAULT_COLOR_SHORTCUT_NAME()),
            shortcut_name_bold: self
                .shortcut_name_bold
                .or(theme.shortcut_name_bold)
                .or(DEFAULT_BOOL_NONE()),
            shortcut_name_italic: self
                .shortcut_name_italic
                .or(theme.shortcut_name_italic)
                .or(DEFAULT_BOOL_NONE()),
            header_fg: self
                .header_fg
                .clone()
                .or(theme.header_fg.clone())
                .or(DEFAULT_COLOR_FG_HEADER()),
            header_bg: self
                .header_bg
                .clone()
                .or(theme.header_bg.clone())
                .or(DEFAULT_COLOR_BG_HEADER()),
            description: self
                .description
                .clone()
                .or(theme.description.clone())
                .or(DEFAULT_COLOR_DESCRIPTION()),
            description_bold: self
                .description_bold
                .or(theme.description_bold)
                .or(DEFAULT_BOOL_NONE()),
            description_italic: self
                .description_italic
                .or(theme.description_italic)
                .or(DEFAULT_BOOL_NONE()),
            free_text_area_bg: self
                .free_text_area_bg
                .clone()
                .or(theme.free_text_area_bg.clone())
                .or(DEFAULT_FREE_TEXT_AREA_BG()),
            home_tilde: self
                .home_tilde
                .clone()
                .or(theme.home_tilde.clone())
                .or(DEFAULT_HOME_TILD()),
            home_tilde_bold: self
                .home_tilde_bold
                .or(theme.home_tilde_bold)
                .or(DEFAULT_BOOL_NONE()),
            home_tilde_italic: self
                .home_tilde_italic
                .or(theme.home_tilde_italic)
                .or(DEFAULT_BOOL_NONE()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ThemeStyles {
    pub title_style: Style,
    pub background_color: Option<Color>,
    pub left_background_color: Option<Color>,
    pub border_color: Option<Color>,
    pub text_style: Style,
    pub text_em_style: Style,
    pub date_style: Style,
    pub path_style: Style,
    pub highlight_color: Option<Color>,
    pub shortcut_name_style: Style,
    pub header_fg_color: Option<Color>,
    pub header_bg_color: Option<Color>,
    pub description_style: Style,
    pub free_text_area_bg_color: Option<Color>,
    pub home_tilde_style: Style,
}

impl ThemeStyles {
    fn build_color(color: Option<&String>) -> Option<Color> { color.map(|c| c.parse().unwrap()) }

    fn build_style(color: Option<&String>, bold: Option<bool>, italic: Option<bool>) -> Style {
        let mut style = Style::new();
        if let Some(color) = Self::build_color(color) {
            style = style.fg(color);
        }
        if let Some(bold) = bold
            && bold
        {
            style = style.add_modifier(Modifier::BOLD);
        }
        if let Some(italic) = italic
            && italic
        {
            style = style.add_modifier(Modifier::ITALIC);
        }
        style
    }

    pub fn from(theme: &Theme) -> ThemeStyles {
        ThemeStyles {
            title_style: Self::build_style(
                theme.title.as_ref(),
                theme.title_bold,
                theme.title_italic,
            ),
            background_color: Self::build_color(theme.background.as_ref()),
            left_background_color: Self::build_color(theme.left_background.as_ref()),
            border_color: Self::build_color(theme.border.as_ref()),
            text_style: Self::build_style(theme.text.as_ref(), theme.text_bold, theme.text_italic),
            text_em_style: Self::build_style(
                theme.text_em.as_ref(),
                theme.text_em_bold,
                theme.text_em_italic,
            ),
            date_style: Self::build_style(theme.date.as_ref(), theme.date_bold, theme.date_italic),
            path_style: Self::build_style(theme.path.as_ref(), theme.path_bold, theme.path_italic),
            highlight_color: Self::build_color(theme.highlight.as_ref()),
            shortcut_name_style: Self::build_style(
                theme.shortcut_name.as_ref(),
                theme.shortcut_name_bold,
                theme.shortcut_name_italic,
            ),
            header_fg_color: Self::build_color(theme.header_fg.as_ref()),
            header_bg_color: Self::build_color(theme.header_bg.as_ref()),
            description_style: Self::build_style(
                theme.description.as_ref(),
                theme.description_bold,
                theme.description_italic,
            ),
            free_text_area_bg_color: Self::build_color(theme.free_text_area_bg.as_ref()),
            home_tilde_style: Self::build_style(
                theme.home_tilde.as_ref(),
                theme.home_tilde_bold,
                theme.home_tilde_italic,
            ),
        }
    }
}

impl Default for ThemeStyles {
    fn default() -> Self { ThemeStyles::from(&Theme::default()) }
}
