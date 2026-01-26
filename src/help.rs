use crossterm::event::{KeyCode, KeyEvent};
use log::debug;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph},
};

use crate::{
    theme::ThemeStyles,
    tui::{EventCaptured, ManagerAction, View, ViewBuilder},
};

pub struct Help {
    styles: ThemeStyles,
}

impl Help {
    pub fn builder(styles: ThemeStyles) -> ViewBuilder {
        ViewBuilder::from(Box::new(Self { styles }))
    }
}

impl View for Help {
    fn handle_key_event(&mut self, key_event: KeyEvent) -> (EventCaptured, ManagerAction) {
        match key_event.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => (
                EventCaptured::Yes,
                ManagerAction::new(false).with_close(true),
            ),
            _ => (EventCaptured::Yes, ManagerAction::new(false)),
        }
    }

    fn draw(&mut self, frame: &mut ratatui::Frame, modal_area: Rect, _active: bool) {
        debug!("Drawing help active");

        let layout = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(15),
            Constraint::Fill(1),
        ]);
        let chunks = layout.split(modal_area);
        let center_layout = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(100),
            Constraint::Fill(1),
        ]);
        let chunks = center_layout.split(chunks[1]);
        let modal_area = chunks[1];

        frame.render_widget(Clear, modal_area);

        let ts = self.styles.text_style;
        let es = self.styles.text_em_style;

        let message = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Use ", ts),
            Span::styled("tab", es),
            Span::styled(" to switch between the views.", ts),
        ]),
        Line::from(vec![
            Span::styled("Use ", ts),
            Span::styled("enter", es),
            Span::styled(" to exit the GUI and go into the selected directory;", ts),
        ]),
        Line::from(vec![
            Span::styled("Use ", ts),
            Span::styled("esc or ctrl+q", es),
            Span::styled(" to simply exit and stay in the current directory.", ts),
        ]),
        Line::from(vec![
            Span::styled("Use the ", ts),
            Span::styled("up", es),
            Span::styled(" and ", ts),
            Span::styled("down", es),
            Span::styled(" arrow keys to select a directory (", ts),
            Span::styled("shift", es),
            Span::styled(" for bigger jumps);", ts),
        ]),
        Line::from(vec![
            Span::styled("Use ", ts),
            Span::styled("page up", es),
            Span::styled(" and ", ts),
            Span::styled("page down", es),
            Span::styled(" to scroll through the list by page;", ts),
        ]),
        Line::from(vec![
            Span::styled("Use ", ts),
            Span::styled("home", es),
            Span::styled(
                " to go to the most recent directory in the history (the top);",
                ts,
            ),
        ]),
        Line::from(vec![
            Span::styled("Use ", ts),
            Span::styled("ctrl+a", es),
            Span::styled(" to see the full directory path without shortcuts, or switch back to shortcut usage.", ts),
        ]),
        Line::from(vec![
            Span::styled("Use ", ts),
            Span::styled("ctrl+d", es),
            Span::styled(" to delete the selected entry.", ts),
        ]),
        Line::from(vec![
            Span::styled("Use ", ts),
            Span::styled("ctrl+e", es),
            Span::styled(" to edit a shortcut description.", ts),
        ]),
        Line::from(vec![
            Span::styled("Use ", ts),
            Span::styled("ctrl+f", es),
            Span::styled(" to switch between exact and fuzzy search.", ts),
        ]),
        Line::from(vec![
            Span::styled("Use ", ts),
            Span::styled("ctrl+h", es),
            Span::styled(" for the help screen.", ts),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("Enter a text to filter.", ts)]),
    ])
    .block(
        Block::default()
            .padding(Padding::new(1, 1, 1, 1))
            .title(Span::styled(" cdir help ", self.styles.title_style))
            .borders(Borders::ALL)
    );

        // Fill the frame with the background color if defined
        if let Some(bg_color) = &self.styles.background_color {
            let background = Paragraph::new("").style(Style::default().bg(*bg_color));
            frame.render_widget(background, modal_area);
        }
        frame.render_widget(message, modal_area);
    }
}
