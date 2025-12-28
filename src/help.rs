use crate::config::Config;
use crossterm::event;
use log::debug;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph};
use ratatui::{DefaultTerminal, Frame};

pub(crate) fn help_run(terminal: &mut DefaultTerminal, config: &Config) {
    debug!("help_run...");
    terminal
        .draw(|frame: &mut Frame| help_draw(frame, config))
        .unwrap();
    event::read().unwrap();
}

fn help_draw(frame: &mut Frame, config: &Config) {
    let ts = config.styles.text_style.clone();
    let es = config.styles.text_em_style.clone();

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
            Span::styled(" to see the full directory path with shortcuts.", ts),
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
            Span::styled("ctrl+h", es),
            Span::styled(" for the help screen.", ts),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("Enter a text to filter.", ts)]),
    ])
    .block(
        Block::default()
            .padding(Padding::new(1, 1, 1, 1))
            .title(Span::styled(
                " cdir help ",
                config.styles.title_style.clone(),
            ))
            .borders(Borders::ALL),
    );
    // Fill the frame with the background color if defined
    if let Some(bg_color) = &config.styles.background_color {
        let background = Paragraph::new("").style(Style::default().bg(bg_color.clone()));
        frame.render_widget(background, frame.area());
    }
    frame.render_widget(message, frame.area());
}
