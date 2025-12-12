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
use std::sync::{Arc, Mutex};

const TABLE_HEADER_LENGTH: usize = 1;
const JUMP_OFFSET: usize = 10;

const DEFAULT_COLOR_DATE: fn() -> String = || String::from("#000080");
const DEFAULT_COLOR_PATH: fn() -> String = || String::from("#000000");
const DEFAULT_COLOR_HIGHLIGHT: fn() -> String = || String::from("#ffe680");
const DEFAULT_COLOR_SHORTCUT_NAME: fn() -> String = || String::from("Green");

const DEFAULT_COLOR_FG_HEADER: fn() -> String = || String::from("White");
const DEFAULT_COLOR_BG_HEADER: fn() -> String = || String::from("#1f2d6c");

const DEFAULT_COLOR_DESCRIPTION: fn() -> String = || String::from("#808080");

const TABLE_COLUMN_SPACING: u16 = 1;
const TABLE_HIGHLIGHT_SYMBOL: &str = "> ";

/// Represents the color configuration for various UI elements.
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

    #[serde(default = "DEFAULT_COLOR_FG_HEADER")]
    pub header_fg: String,

    #[serde(default = "DEFAULT_COLOR_BG_HEADER")]
    pub header_bg: String,

    #[serde(default = "DEFAULT_COLOR_DESCRIPTION")]
    pub description: String,
}

impl Default for Colors {
    fn default() -> Self {
        Colors {
            date: DEFAULT_COLOR_DATE(),
            path: DEFAULT_COLOR_PATH(),
            highlight: DEFAULT_COLOR_HIGHLIGHT(),
            shortcut_name: DEFAULT_COLOR_SHORTCUT_NAME(),
            header_fg: DEFAULT_COLOR_FG_HEADER(),
            header_bg: DEFAULT_COLOR_BG_HEADER(),
            description: DEFAULT_COLOR_DESCRIPTION(),
        }
    }
}

/// Represents the possible results of a GUI action.
pub enum GuiResult {
    Quit,
    Print(String),
    Next,
    Help,
}

/// A function type that converts a vector of items of type T into a vector of table rows.
pub type RowifyFn<'store, T> = Box<dyn for<'a> Fn(&'a [T], &[u16]) -> Vec<Row<'a>> + 'store>;

/// A function type that deletes an item of type T into the store
pub type DeleteFn<'store, T> = Box<dyn Fn(&T) + 'store>;

// A trait for handling modal views
pub trait ModalView<T: Clone> {
    fn initialize(&mut self, item: &T);
    fn handle_event(&mut self, event: Event) -> bool;
    fn draw(&mut self, frame: &mut Frame);
}

/// A generic table view for displaying data in a tabular format within the GUI.
pub struct TableView<'store, T: Clone, S> {
    data_model: DataViewModel<'store, T>,
    column_names: Vec<String>,
    column_constraints: Vec<Constraint>,
    table_state: TableState,
    table_rows_count: u16, // Number of lines in the table, excluding header & footer
    rowify: RowifyFn<'store, T>,
    stringify: fn(&T) -> String,
    search_string: Arc<Mutex<String>>,
    colors: Colors,
    view_state: Rc<RefCell<S>>,
    delete_fn: DeleteFn<'store, T>,
    modal_view: Option<Box<dyn ModalView<T>>>,
    modal_active: bool,
}

impl<'store, T: Clone> TableView<'store, T, bool> {
    /// Create a new TableView instance.
    ///
    /// ### Parameters
    /// - `column_names`: A vector of strings representing the names of the table columns.
    /// - `list_fn`: A boxed function that lists items of type T from the store
    /// - `rowify`: A boxed function that converts a vector of items of type T into a vector of table rows.
    /// - `stringify`: A function that converts an item of type T into a string
    /// - `config`: A reference to the configuration object containing color settings.
    /// - `view_state`: A reference-counted, mutable boolean indicating the current view state.
    /// - `delete_fn`: A boxed function that deletes an item of type T from the store
    ///
    /// ### Returns
    /// A new instance of `TableView`.
    pub(crate) fn new(
        column_names: Vec<String>,
        column_constraints: Vec<Constraint>,
        list_fn: Box<ListFunction<'store, T>>,
        rowify: RowifyFn<'store, T>,
        stringify: fn(&T) -> String,
        config: &Config,
        view_state: Rc<RefCell<bool>>,
        delete_fn: DeleteFn<'store, T>,
        search_string: Arc<Mutex<String>>,
        modal_view: Option<Box<dyn ModalView<T>>>,
    ) -> Self {
        TableView {
            data_model: DataViewModel::new(list_fn),
            column_names,
            column_constraints,
            table_state: TableState::default(),
            table_rows_count: 0,
            rowify,
            stringify,
            search_string,
            colors: config.colors.clone(),
            view_state,
            delete_fn,
            modal_view,
            modal_active: false,
        }
    }

    /// Get the index of the currently selected row, if any.
    fn selected_row(&self) -> Option<usize> {
        let selected = self.table_state.selected_cell();
        selected.map(|pos| pos.0)
    }

    /// Run the GUI.
    pub(crate) fn run(&mut self, terminal: &mut DefaultTerminal) -> GuiResult {
        debug!("Select...");
        self.table_state.select_cell(Some((0, 0)));
        debug!("Selected");
        let _ = terminal.draw(|frame| self.draw(frame));

        loop {
            let event = event::read().unwrap();
            debug!("Main loop event: {:?}", event);

            if self.modal_active {
                if !self.modal_view.as_mut().unwrap().handle_event(event) {
                    self.modal_active = false;
                    self.data_model.reload();
                }
                let _ = terminal.draw(|frame| self.draw(frame));
                continue;
            }

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
                                self.search_string.lock().unwrap().as_str(),
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
                            let mut search_string = self.search_string.lock().unwrap();
                            search_string.pop();
                            self.data_model.update(
                                0,
                                self.table_rows_count,
                                search_string.as_str(),
                                true,
                            );
                        }
                        KeyCode::Char(c) => {
                            if key.modifiers != KeyModifiers::CONTROL {
                                let mut search_string = self.search_string.lock().unwrap();
                                search_string.push(c);
                                self.data_model.update(
                                    0,
                                    self.table_rows_count,
                                    search_string.as_str(),
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
                                    'e' => self.handle_modal_event(),
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

    /// Handle the chosen item and return its string representation.
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

    /// Handle moving the selection down in the table.
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
                    self.search_string.lock().unwrap().as_str(),
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

    /// Handle moving the selection up in the table.
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
                    self.search_string.lock().unwrap().as_str(),
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

    /// Handle deleting the currently selected item.
    fn handle_delete(&mut self) {
        debug!("handle_delete");
        if let Some(items) = &self.data_model.entries {
            let current_row = self.selected_row();
            (self.delete_fn)(&items[current_row.unwrap()]);
            self.data_model.reload();
        }
    }

    fn handle_modal_event(&mut self) {
        debug!("handle_modal_event");
        let mut current_row: usize = 0;
        if let Some(items) = &self.data_model.entries {
            current_row = match self.selected_row() {
                Some(row) => row,
                None => {
                    debug!("No row selected");
                    return;
                }
            };
        }
        if let Some(modal) = &mut self.modal_view {
            if let Some(items) = &self.data_model.entries {
                modal.initialize(&items[current_row]);
                self.modal_active = true;
            }
        }
    }

    /// Draw the table view on the given frame.
    fn draw(&mut self, frame: &mut Frame) {
        let vertical = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).spacing(0);
        let [main, input] = vertical.areas(frame.area());
        self.table_rows_count = main.height - TABLE_HEADER_LENGTH as u16;
        debug!("self.table_rows_count={}", self.table_rows_count);

        let search_string_lock = self.search_string.lock().unwrap();
        let search_string = search_string_lock.clone();
        drop(search_string_lock);

        if self.data_model.length != self.table_rows_count {
            self.data_model.update(
                self.data_model.first,
                main.height - TABLE_HEADER_LENGTH as u16,
                search_string.as_str(),
                true,
            );
        } else {
            self.data_model.update(
                self.data_model.first,
                self.table_rows_count,
                search_string.as_str(),
                true,
            );
        }

        self.render_table(frame, main);

        let horizontal =
            Layout::horizontal([Constraint::Percentage(90), Constraint::Percentage(10)]).spacing(0);
        let [left, right] = horizontal.areas(input);

        let pa = Paragraph::new(format!("> {}", search_string.as_str()))
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

        if self.modal_active {
            if let Some(modal) = &mut self.modal_view {
                modal.draw(frame);
            }
        }
    }

    fn resolve_column_widths(constraints: &[Constraint], total_width: u16) -> Vec<u16> {
        use ratatui::layout::Constraint::*;
        let mut widths = vec![0; constraints.len()];
        let mut remaining_width = total_width as i32;

        // First, assign fixed lengths
        for (i, c) in constraints.iter().enumerate() {
            if let Length(l) = c {
                widths[i] = *l;
                remaining_width -= *l as i32;
            }
        }

        let fill_total: usize = constraints
            .iter()
            .filter_map(|c| {
                if let Fill(val) = c {
                    Some(*val as usize)
                } else {
                    None
                }
            })
            .sum();
        debug!(
            "remaining_width={} fill_total={}",
            remaining_width, fill_total
        );
        if fill_total > 0 && remaining_width > 0 {
            for (i, c) in constraints.iter().enumerate() {
                if let Fill(f) = c {
                    widths[i] = (f64::from(*f) / f64::from(fill_total as u16)
                        * f64::from(remaining_width))
                    .round() as u16;
                }
            }
        }

        // Handle Percentage constraints
        for (i, c) in constraints.iter().enumerate() {
            if let Percentage(p) = c {
                widths[i] = ((total_width as u32 * *p as u32) / 100) as u16;
            }
        }

        widths
    }

    /// Render a table with some rows and columns.
    pub fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        trace!(
            "render_table data_first={} data_length={}",
            self.data_model.first,
            self.data_model.length
        );

        let actual_width = Self::resolve_column_widths(
            &self.column_constraints,
            area.width - TABLE_HIGHLIGHT_SYMBOL.len() as u16 - TABLE_COLUMN_SPACING * 2,
        );
        debug!("area widht={} col_width={:?}", area.width, actual_width);

        let rows: Vec<Row> = self
            .data_model
            .entries
            .as_ref()
            .map_or(vec![], |entries| (self.rowify)(entries, &actual_width));

        let table = Table::new(rows, self.column_constraints.clone())
            .header(
                Row::new(self.column_names.clone()).style(
                    Style::new()
                        .fg(self.colors.header_fg.parse().unwrap())
                        .bg(self.colors.header_bg.parse().unwrap())
                        .bold(),
                ),
            )
            .column_spacing(TABLE_COLUMN_SPACING)
            .style(Color::Black)
            .row_highlight_style(
                Style::new()
                    .black()
                    .bg(self.colors.highlight.parse().unwrap())
                    .bold(),
            )
            .highlight_symbol(TABLE_HIGHLIGHT_SYMBOL);

        if self.selected_row().is_none() && self.data_model.length > 0 {
            self.table_state.select(Some(0));
            debug!("No row selected: select 0")
        }

        frame.render_stateful_widget(table, area, &mut self.table_state);
    }
}
