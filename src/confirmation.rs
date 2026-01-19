use crossterm::event::{KeyCode, KeyEvent};
use log::debug;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::{
    theme::ThemeStyles,
    tui::{EventCaptured, ManagerAction, View, ViewBuilder, ViewManager},
};

pub struct Confirmation {
    styles: ThemeStyles,
    pub message: String,
    selected: ConfirmationButton,
    result: bool,
}

#[derive(Copy, Clone, PartialEq)]
enum ConfirmationButton {
    Yes,
    Cancel,
}

impl Confirmation {
    pub fn builder(message: String, styles: ThemeStyles) -> ViewBuilder {
        ViewBuilder::from(Box::new(Self {
            styles,
            message,
            selected: ConfirmationButton::Yes,
            result: false,
        }))
    }

    pub fn is_yes(&self) -> bool { self.result }
}

impl View for Confirmation {
    fn draw(&mut self, frame: &mut ratatui::Frame, _area: Rect, _active: bool) {
        debug!("Drawing confirmation");

        // Calculate max line length of the message
        let lines: Vec<&str> = self.message.lines().collect();
        let max_line_len = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        let min_width = 32; // Minimum width for UI/buttons
        let padding = 8; // Padding for left/right
        let modal_width = std::cmp::max(min_width, max_line_len + padding) as u16;
        let modal_height = 7;

        let modal_area = ViewManager::centered_rect(frame.area(), modal_width, modal_height);

        frame.render_widget(Clear, modal_area);
        // Optional: background color for modal
        if let Some(bg_color) = &self.styles.background_color {
            let background = Paragraph::new("").style(Style::default().bg(*bg_color));
            frame.render_widget(background, modal_area);
        }

        let block = Block::default()
            .title("Confirmation")
            .title_style(self.styles.title_style)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.styles.border_color.unwrap()))
            .style(self.styles.text_style);
        frame.render_widget(block, modal_area);

        // Split modal into title, message, buttons
        let inner = ratatui::layout::Rect {
            x: modal_area.x + 1,
            y: modal_area.y + 1,
            width: modal_area.width - 2,
            height: modal_area.height - 2,
        };
        let vchunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // title (empty, since Block has title)
                Constraint::Fill(2),   // message
                Constraint::Length(1), // buttons
            ])
            .split(inner);

        // Multi-line message in the middle, centered
        let message_height = vchunks[1].height as usize;
        let msg_lines = lines.len();
        let mut text = String::new();
        if msg_lines < message_height {
            // Add blank lines above to center vertically
            let pad = (message_height - msg_lines) / 2;
            for _ in 0..pad {
                text.push('\n');
            }
        }
        text.push_str(&self.message);
        let msg = Paragraph::new(text).alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(msg, vchunks[1]);

        // Buttons at the bottom
        let button_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(10),
                Constraint::Fill(1),
                Constraint::Length(10),
            ])
            .split(vchunks[2]);
        let yes_style = if self.selected == ConfirmationButton::Yes {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        let cancel_style = if self.selected == ConfirmationButton::Cancel {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Red)
        };
        let yes = Paragraph::new(" Yes ")
            .style(yes_style)
            .alignment(ratatui::layout::Alignment::Center);
        let cancel = Paragraph::new(" Cancel ")
            .style(cancel_style)
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(yes, button_layout[0]);
        frame.render_widget(cancel, button_layout[2]);
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> (EventCaptured, ManagerAction) {
        let mut redraw = false;
        let mut close = false;
        match key_event.code {
            KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
                // Toggle selection
                self.selected = match self.selected {
                    ConfirmationButton::Yes => ConfirmationButton::Cancel,
                    ConfirmationButton::Cancel => ConfirmationButton::Yes,
                };
                redraw = true;
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.result = self.selected == ConfirmationButton::Yes;
                close = true;
            }
            _ => {}
        }
        (
            EventCaptured::Yes,
            ManagerAction::new(redraw).with_close(close),
        )
    }
}
