use std::{
    rc::Rc,
    sync::{Arc, Mutex},
};

use crossterm::event::KeyEvent;
use log::debug;
use ratatui::layout::{Constraint, Layout, Rect};

use crate::{
    config::Config,
    config_button::ConfigButton,
    list_indicator_view::ListIndicatorView,
    model::ListFunction,
    search_text_view::{SearchTextState, SearchTextView},
    store::Path,
    tableview::{DeleteFn, EditorViewBuilder, RowifyFn, TableView, TableViewState},
    tui::{EventCaptured, ManagerAction, View, ViewBuilder, ViewManager},
};

const PATH_HISTORY_VIEW_ID: u16 = 0;
const SEARCH_TEXT_VIEW_1: u16 = 1;
const CONFIGURATION_VIEW: u16 = 2;
const LIST_INDICATOR_VIEW: u16 = 3;

pub struct HistoryViewContainer {}

impl HistoryViewContainer {
    pub fn builder(
        vm: Rc<ViewManager>,
        column_names: Vec<String>,
        column_constraints: Vec<Constraint>,
        list_fn: Box<ListFunction<Path>>,
        rowify: RowifyFn<Path>,
        stringify: fn(&Path) -> String,
        config: Arc<Mutex<Config>>,
        view_state: Arc<Mutex<TableViewState>>,
        delete_fn: DeleteFn<Path>,
        editor_modal_view_builder: Option<EditorViewBuilder<Path>>,
        search_text_state: Arc<Mutex<SearchTextState>>,
    ) -> ViewBuilder {
        ViewBuilder::from(Box::new(Self {}))
            .child(
                PATH_HISTORY_VIEW_ID,
                TableView::builder(
                    vm.clone(),
                    "path".to_string(),
                    column_names,
                    column_constraints,
                    list_fn,
                    rowify,
                    stringify,
                    config.clone(),
                    view_state,
                    delete_fn,
                    editor_modal_view_builder,
                    Box::new(|paths| paths.iter().position(|p| !p.smart_path).unwrap_or(0)),
                )
                .with_publish_events(true),
            )
            .child(
                SEARCH_TEXT_VIEW_1,
                SearchTextView::builder(config.clone(), search_text_state.clone()),
            )
            .child(
                CONFIGURATION_VIEW,
                ConfigButton::builder(vm.clone(), config.clone()),
            )
            .child(
                LIST_INDICATOR_VIEW,
                ListIndicatorView::builder(vm.clone(), config.clone(), "path".to_string()),
            )
    }
}

impl View for HistoryViewContainer {
    fn capture_focus(&self) -> bool { true }
    fn broadcast_keyboard_events(&self) -> bool { true }
    fn resize(&mut self, area: Rect) -> Vec<(u16, Rect)> {
        let vertical = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).spacing(0);
        let [main, bottom] = vertical.areas(area);

        let horizontal = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(14),
            Constraint::Length(1),
            Constraint::Length(10),
        ])
        .spacing(0);
        let [search_text_area, list_indicator_rect, _, config_rect] = horizontal.areas(bottom);

        vec![
            (PATH_HISTORY_VIEW_ID, main),
            (SEARCH_TEXT_VIEW_1, search_text_area),
            (CONFIGURATION_VIEW, config_rect),
            (LIST_INDICATOR_VIEW, list_indicator_rect),
        ]
    }
    fn draw(&mut self, _frame: &mut ratatui::Frame, area: ratatui::prelude::Rect, active: bool) {
        // nothing to draw
        debug!("draw area='{}' active='{}", area, active);
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> (EventCaptured, ManagerAction) {
        let _ = key_event;
        debug!("handle_key_event");
        (EventCaptured::No, ManagerAction::new(false))
    }
}
