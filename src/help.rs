use crossterm::event;
use log::debug;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph};
use ratatui::{style::Color, DefaultTerminal, Frame};

pub(crate) fn help_run(terminal: &mut DefaultTerminal) {
    debug!("help_run...");
    terminal.draw(help_draw).unwrap();
    event::read().unwrap();
}

fn help_draw(frame: &mut Frame) {
    let message = Paragraph::new(vec![
        Line::from(vec![
            Span::raw("Use "),
            Span::styled("tab", Style::default().fg(Color::Cyan)),
            Span::raw(" to switch between the views."),
        ]),
        Line::from(vec![
            Span::raw("Use "),
            Span::styled("enter", Style::default().fg(Color::Cyan)),
            Span::raw(" to exit the GUI and go into the selected directory;"),
        ]),
        Line::from(vec![
            Span::raw("Use "),
            Span::styled("esc or ctrl+q", Style::default().fg(Color::Cyan)),
            Span::raw(" to simply exit and stay in the current directory."),
        ]),
        Line::from(vec![
            Span::raw("Use the "),
            Span::styled("up", Style::default().fg(Color::Cyan)),
            Span::raw(" and "),
            Span::styled("down", Style::default().fg(Color::Cyan)),
            Span::raw(" arrow keys to select a directory ("),
            Span::styled("shift", Style::default().fg(Color::Cyan)),
            Span::raw(" for bigger jumps);"),
        ]),
        Line::from(vec![
            Span::raw("Use "),
            Span::styled("page up", Style::default().fg(Color::Cyan)),
            Span::raw(" and "),
            Span::styled("page down", Style::default().fg(Color::Cyan)),
            Span::raw(" to scroll through the list by page;"),
        ]),
        Line::from(vec![
            Span::raw("Use "),
            Span::styled("home", Style::default().fg(Color::Cyan)),
            Span::raw(" to go to the most recent directory in the history (the top);"),
        ]),
        Line::from(vec![
            Span::raw("Use "),
            Span::styled("ctrl+a", Style::default().fg(Color::Cyan)),
            Span::raw(" to see the full directory path with shortcuts."),
        ]),
        Line::from(vec![
            Span::raw("Use "),
            Span::styled("ctrl+d", Style::default().fg(Color::Cyan)),
            Span::raw(" to delete the selected entry."),
        ]),
        Line::from(vec![
            Span::raw("Use "),
            Span::styled("ctrl+e", Style::default().fg(Color::Cyan)),
            Span::raw(" to edit a shortcut description."),
        ]),
        Line::from(vec![
            Span::raw("Use "),
            Span::styled("ctrl+h", Style::default().fg(Color::Cyan)),
            Span::raw(" for the help screen."),
        ]),
        Line::from(vec![Span::raw("Enter a text to filter.")]),
    ])
    .block(
        Block::default()
            .padding(Padding::new(1, 1, 1, 1))
            .title(Span::styled(
                " cdir help ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ))
            .borders(Borders::ALL),
    );
    frame.render_widget(message, frame.area());
}
