use crate::config::Config;
use crate::model::{DataViewModel, ListFunction};
use crossterm::event;
use crossterm::event::{Event, KeyCode, KeyModifiers};
use log::{debug, trace, warn};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::prelude::{Color, Style};
use ratatui::style::Stylize;
use ratatui::widgets::{Paragraph, Row, Table, TableState};
use ratatui::{DefaultTerminal, Frame};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;

const TABLE_HEADER_LENGTH: usize = 1;
const JUMP_OFFSET: usize = 10;

const DEFAULT_COLOR_DATE: fn() -> String = || String::from("#000080");
const DEFAULT_COLOR_PATH: fn() -> String = || String::from("#000000");
const DEFAULT_COLOR_HIGHLIGHT: fn() -> String = || String::from("#FFDD51");
const DEFAULT_COLOR_SHORTCUT_NAME: fn() -> String = || String::from("Green");

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Colors {
    #[serde(default = "DEFAULT_COLOR_DATE")]
    pub date: String,

    #[serde(default = "DEFAULT_COLOR_PATH")]
    pub path: String,

    #[serde(default = "DEFAULT_COLOR_HIGHLIGHT")]
    pub highlight: String,

    #[serde(default = "DEFAULT_COLOR_SHORTCUT_NAME")]
    pub shortcut_name: String,
}

/// Represents the possible results of a GUI action.
pub enum GuiResult {
    Quit,
    Print(String),
    Next,
    Help,
}

/// A function type that converts a vector of items of type T into a vector of table rows.
pub type RowifyFn<'store, T> = Box<dyn Fn(&Vec<T>) -> Vec<Row> + 'store>;

/// A function type that deletes an item of type T into the store
pub type DeleteFn<'store, T> = Box<dyn Fn(&T) + 'store>;

/// A generic table view for displaying data in a tabular format within the GUI.
pub struct TableView<'store, T: Clone, S> {
    data_model: DataViewModel<'store, T>,
    column_names: Vec<String>,
    table_state: TableState,
    table_rows_count: u16, // Number of lines in the table, excluding header & footer
    rowify: RowifyFn<'store, T>,
    stringify: fn(&T) -> String,
    search_string: String,
    colors: Colors,
    view_state: Rc<RefCell<S>>,
    delete_fn: DeleteFn<'store, T>,
}

impl<'store, T: Clone> TableView<'store, T, bool> {
    pub(crate) fn new(
        column_names: Vec<String>,
        list_fn: Box<ListFunction<'store, T>>,
        rowify: RowifyFn<'store, T>,
        stringify: fn(&T) -> String,
        config: &Config,
        view_state: Rc<RefCell<bool>>,
        delete_fn: DeleteFn<'store, T>,
    ) -> Self {
        TableView {
            data_model: DataViewModel::new(list_fn),
            column_names,
            table_state: TableState::default(),
            table_rows_count: 0,
            rowify,
            stringify,
            search_string: String::new(),
            colors: config.colors.clone(),
            view_state,
            delete_fn,
        }
    }

    fn selected_row(&self) -> Option<usize> {
        let selected = self.table_state.selected_cell();
        selected.map(|pos| pos.0)
    }

    /// Run the application.
    pub(crate) fn run(&mut self, terminal: &mut DefaultTerminal) -> GuiResult {
        debug!("Select...");
        self.table_state.select_cell(Some((0, 0)));
        debug!("Selected");
        let _ = terminal.draw(|frame| self.draw(frame));
        loop {
            let event = event::read().unwrap();
            match event {
                Event::Key(key) => {
                    match key.code {
                        KeyCode::Enter => {
                            break self
                                .handle_chosen()
                                .map_or(GuiResult::Quit, GuiResult::Print)
                        }
                        KeyCode::Home => {
                            self.data_model.update(
                                0,
                                self.table_rows_count,
                                self.search_string.as_str(),
                                true,
                            );
                            self.table_state.select_cell(Some((0, 0)))
                        }
                        KeyCode::Down => {
                            self.handle_down(key.modifiers.contains(KeyModifiers::SHIFT), false);
                        }
                        KeyCode::Up => {
                            self.handle_up(key.modifiers.contains(KeyModifiers::SHIFT), false);
                        }
                        KeyCode::PageDown => {
                            self.handle_down(key.modifiers.contains(KeyModifiers::SHIFT), true);
                        }
                        KeyCode::PageUp => {
                            self.handle_up(key.modifiers.contains(KeyModifiers::SHIFT), true);
                        }
                        KeyCode::Tab => break GuiResult::Next,
                        KeyCode::Esc => break GuiResult::Quit,
                        KeyCode::Backspace => {
                            self.search_string.pop();
                            self.data_model.update(
                                0,
                                self.table_rows_count,
                                self.search_string.as_str(),
                                true,
                            );
                        }
                        KeyCode::Char(c) => {
                            if key.modifiers != KeyModifiers::CONTROL {
                                self.search_string.push(c);
                                self.data_model.update(
                                    0,
                                    self.table_rows_count,
                                    self.search_string.as_str(),
                                    true,
                                );
                            } else {
                                match c {
                                    'q' => break GuiResult::Quit,
                                    'h' => {
                                        debug!("Help");
                                        break GuiResult::Help;
                                    }
                                    'a' => {
                                        let s = *self.view_state.borrow();
                                        *self.view_state.borrow_mut() = !s
                                    }
                                    'd' => self.handle_delete(),
                                    _ => {}
                                }
                            }
                        }
                        _ => {
                            warn!("Unknown action key={}", key.code);
                        }
                    }
                    let _ = terminal.draw(|frame| self.draw(frame));
                }
                Event::Mouse(mouse_event) => {
                    debug!("Mouse event: {:?}", mouse_event);
                }
                Event::Resize(width, height) => {
                    debug!("Resize event: width={}, height={}", width, height);
                    let _ = terminal.draw(|frame| self.draw(frame));
                }
                _ => {
                    debug!("Other event: {:?}", event);
                }
            }
        }
    }

    fn handle_chosen(&self) -> Option<String> {
        debug!("handle_chosen");
        if let Some(items) = &self.data_model.entries {
            let current_row = self.selected_row();
            current_row.map(|row| (self.stringify)(&items[row]))
        } else {
            warn!("No data!");
            None
        }
    }

    fn handle_down(&mut self, jump: bool, page: bool) {
        if self.data_model.entries.is_none() {
            debug!("No data");
            return;
        }
        let current_row = self.selected_row();
        if let Some(current_row) = current_row {
            let mut offset = if jump { JUMP_OFFSET } else { 1 };
            offset = if page {
                self.table_rows_count as usize
            } else {
                offset
            };
            debug!(
                "current row={} length={}",
                current_row, self.data_model.length
            );
            if (current_row == (self.table_rows_count - 1) as usize) || page {
                self.data_model.update_to_offset(
                    offset as i64,
                    self.table_rows_count,
                    self.search_string.as_str(),
                );
            }
            let mut next = current_row + offset;
            if next >= self.data_model.length as usize {
                next = (self.data_model.length - 1) as usize;
            }
            self.table_state.select(Some(next));
        } else {
            debug!("no current row");
        }
    }

    fn handle_up(&mut self, jump: bool, page: bool) {
        if self.data_model.entries.is_none() {
            debug!("No data");
            return;
        }
        let current_row = self.selected_row();
        if let Some(current_row) = current_row {
            let mut offset = if jump { JUMP_OFFSET } else { 1 };
            offset = if page {
                self.table_rows_count as usize
            } else {
                offset
            };
            debug!(
                "current row={} length={}",
                current_row, self.data_model.length
            );
            if (current_row == 0) || page {
                self.data_model.update_to_offset(
                    -(offset as i64),
                    self.table_rows_count,
                    self.search_string.as_str(),
                );
            }
            let mut next: i64 = (current_row as i64) - offset as i64;
            if next < 0 {
                next = 0;
            }
            self.table_state.select(Some(next as usize));
        } else {
            debug!("no current row");
        }
    }

    fn handle_delete(&mut self) {
        debug!("handle_delete");
        if let Some(items) = &self.data_model.entries {
            let current_row = self.selected_row();
            (self.delete_fn)(&items[current_row.unwrap()]);
            self.data_model.reload();
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        let vertical = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).spacing(0);
        let [main, input] = vertical.areas(frame.area());
        self.table_rows_count = main.height - TABLE_HEADER_LENGTH as u16;
        debug!("self.table_rows_count={}", self.table_rows_count);
        if self.data_model.length != self.table_rows_count {
            self.data_model.update(
                self.data_model.first,
                main.height - TABLE_HEADER_LENGTH as u16,
                self.search_string.as_str(),
                true,
            );
        }
        self.render_table(frame, main);

        let horizontal =
            Layout::horizontal([Constraint::Percentage(90), Constraint::Percentage(10)]).spacing(0);
        let [left, right] = horizontal.areas(input);

        let pa = Paragraph::new(format!("> {}", self.search_string))
            .style(Style::default().fg(self.colors.path.parse::<Color>().unwrap()));
        frame.render_widget(pa, left);

        let pb = if self.data_model.length > 0 {
            Paragraph::new("")
                .style(Style::default().fg(Color::Black))
                .alignment(Alignment::Center)
        } else {
            Paragraph::new("no entry")
                .style(Style::default().fg(Color::Black))
                .bg(Color::Red)
                .alignment(Alignment::Center)
        };

        frame.render_widget(pb, right);
    }

    /// Render a table with some rows and columns.
    pub fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        trace!(
            "render_table data_first={} data_length={}",
            self.data_model.first,
            self.data_model.length
        );
        let rows: Vec<Row> = self
            .data_model
            .entries
            .as_ref()
            .map_or(vec![], |entries| (self.rowify)(entries));

        let widths = [Constraint::Length(20), Constraint::Fill(1)];

        let table = Table::new(rows, widths)
            .header(
                Row::new(self.column_names.clone()).style(
                    Style::new()
                        .fg(Color::White)
                        .bg(Color::Rgb(0, 0x33, 0x66))
                        .bold(),
                ),
            )
            .column_spacing(1)
            .style(Color::Black)
            .row_highlight_style(
                Style::new()
                    .black()
                    .bg(self.colors.highlight.parse().unwrap())
                    .bold(),
            )
            .highlight_symbol("> ");

        if self.selected_row().is_none() && self.data_model.length > 0 {
            self.table_state.select(Some(0));
            debug!("No row selected: select 0")
        }

        frame.render_stateful_widget(table, area, &mut self.table_state);
    }
}
