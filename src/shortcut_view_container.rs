use std::{
    rc::Rc,
    sync::{Arc, Mutex},
};

use crossterm::event::KeyEvent;
use log::debug;
use ratatui::layout::{Constraint, Layout, Rect};

use crate::{
    config::Config,
    list_indicator_view::ListIndicatorView,
    model::ListFunction,
    search_text_view::{SearchTextState, SearchTextView},
    store::Shortcut,
    tableview::{DeleteFn, EditorViewBuilder, RowifyFn, TableView, TableViewState},
    tui::{EventCaptured, ManagerAction, View, ViewBuilder, ViewManager},
};

const SHORTCUT_VIEW_ID: u16 = 0;
const SEARCH_TEXT_VIEW_1: u16 = 1;
const LIST_INDICATOR_VIEW: u16 = 2;

pub struct ShortcutViewContainer {}

impl ShortcutViewContainer {
    pub fn builder(
        vm: Rc<ViewManager>,
        column_names: Vec<String>,
        column_constraints: Vec<Constraint>,
        list_fn: Box<ListFunction<Shortcut>>,
        rowify: RowifyFn<Shortcut>,
        stringify: fn(&Shortcut) -> String,
        config: Arc<Mutex<Config>>,
        view_state: Arc<Mutex<TableViewState>>,
        delete_fn: DeleteFn<Shortcut>,
        editor_modal_view_builder: Option<EditorViewBuilder<Shortcut>>,
        search_text_state: Arc<Mutex<SearchTextState>>,
    ) -> ViewBuilder {
        ViewBuilder::from(Box::new(Self {}))
            .child(
                SHORTCUT_VIEW_ID,
                TableView::builder(
                    vm,
                    "shortcut".to_string(),
                    column_names,
                    column_constraints,
                    list_fn,
                    rowify,
                    stringify,
                    config.clone(),
                    view_state,
                    delete_fn,
                    editor_modal_view_builder,
                    Box::new(|_| 0),
                )
                .with_publish_events(true),
            )
            .child(
                SEARCH_TEXT_VIEW_1,
                SearchTextView::builder(config.clone(), search_text_state.clone()),
            )
            .child(
                LIST_INDICATOR_VIEW,
                ListIndicatorView::builder(config.clone(), "shortcut".to_string()),
            )
    }
}

impl View for ShortcutViewContainer {
    fn capture_focus(&self) -> bool { true }
    fn broadcast_keyboard_events(&self) -> bool { true }
    fn resize(&mut self, area: Rect) -> Vec<(u16, Rect)> {
        let vertical = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).spacing(0);
        let [main, bottom] = vertical.areas(area);

        let horizontal =
            Layout::horizontal([Constraint::Fill(1), Constraint::Length(14)]).spacing(0);
        let [search_text_area, right] = horizontal.areas(bottom);

        vec![
            (SHORTCUT_VIEW_ID, main),
            (SEARCH_TEXT_VIEW_1, search_text_area),
            (LIST_INDICATOR_VIEW, right),
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
