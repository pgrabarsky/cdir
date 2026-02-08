use std::{cell::RefCell, collections::HashSet, ops::Add, rc::Rc};

use crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode, KeyEvent, KeyModifiers,
    MouseEvent,
};
use log::{debug, info, trace, warn};
use ratatui::layout::{Position, Rect};
use tokio::{
    select,
    sync::{broadcast, broadcast::error::RecvError},
};
use tokio_stream::StreamExt;

use crate::tui::{
    ViewBuilder,
    event::{GenericEvent, ViewManagerEvent},
    managed_view::ManagedView,
    view::{EventCaptured, ManagerAction, View},
};

#[cfg(test)]
#[path = "view_manager_tests.rs"]
mod view_manager_tests;

type ModalCallBack = Box<dyn FnOnce(&mut dyn View, &dyn View) -> ManagerAction>;
type HelpViewBuilderCallBack = Box<dyn Fn() -> ViewBuilder>;

/// Represents a modal view entry with its associated parent and close callback.
struct ModalEntry {
    /// The modal view itself
    modal_view: ManagedView,
    /// Reference to the parent view that opened this modal
    parent_view: Option<Rc<RefCell<ManagedView>>>,
    /// Optional callback to invoke when the modal is closed
    on_close: Option<ModalCallBack>,
}

pub struct ViewManager {
    tx: broadcast::Sender<GenericEvent>,

    views: RefCell<Vec<Rc<RefCell<ManagedView>>>>,
    top_level_view_idx: RefCell<usize>,

    receive_events_views: RefCell<Vec<Rc<RefCell<ManagedView>>>>,
    active_view: RefCell<Vec<Option<Vec<Rc<RefCell<ManagedView>>>>>>,
    modal_views: RefCell<Vec<Rc<RefCell<ModalEntry>>>>,
    context_view: RefCell<Option<Rc<RefCell<ManagedView>>>>,

    global_help_view_builder_cb: Option<HelpViewBuilderCallBack>,

    exit_string: RefCell<Option<String>>,
}

#[allow(unused)]
impl Default for ViewManager {
    fn default() -> Self { Self::new() }
}

#[allow(unused)]
impl ViewManager {
    pub fn new() -> ViewManager {
        ViewManager {
            tx: broadcast::channel::<GenericEvent>(16).0,
            views: RefCell::new(vec![]),
            top_level_view_idx: RefCell::new(0),
            receive_events_views: RefCell::new(vec![]),
            active_view: RefCell::new(vec![]),
            modal_views: RefCell::new(vec![]),
            context_view: RefCell::new(None),
            global_help_view_builder_cb: None,
            exit_string: RefCell::new(None),
        }
    }

    pub fn tx(&self) -> broadcast::Sender<GenericEvent> { self.tx.clone() }

    pub fn set_global_help_view(&mut self, help_view: HelpViewBuilderCallBack) {
        self.global_help_view_builder_cb = Some(help_view);
    }

    /// Returns a centered rectangle of the specified width and height within the given area.
    ///
    /// If the requested width or height is larger than the area, it will be clamped
    /// to fit within the area's dimensions.
    ///
    /// # Arguments
    /// * `area` - The containing rectangle
    /// * `width` - The desired width of the centered rectangle
    /// * `height` - The desired height of the centered rectangle
    ///
    /// # Returns
    /// A `Rect` centered within the area with the specified dimensions
    pub fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
        let width = width.min(area.width);
        let height = height.min(area.height);

        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;

        Rect::new(x, y, width, height)
    }

    pub fn show_modal_generic(&self, v: ViewBuilder, callback: Option<ModalCallBack>) {
        debug!("show_modal_generic");
        let mut modal_view = v.build();

        // modal are initialized on the fly...
        self.init_view_tree(&mut modal_view);

        let entry = ModalEntry {
            modal_view,
            parent_view: self.context_view.borrow().as_ref().map(|rc| rc.clone()),
            on_close: callback,
        };
        self.modal_views
            .borrow_mut()
            .push(Rc::new(RefCell::new(entry)));
    }

    fn init_view_tree(&self, mv: &mut ManagedView) {
        mv.view.init();
        for child in mv.children.iter() {
            let mut child = child.borrow_mut();
            self.init_view_tree(&mut child);
        }
    }

    /// Shows a modal with a type-safe callback that receives the parent view with its concrete type.
    ///
    /// This method provides a generic interface that eliminates the need for downcasting in user code.
    /// The framework performs the downcast internally, exposing only a type-safe callback to the caller.
    ///
    /// # Type Parameters
    /// * `P` - The concrete type of the parent view (automatically inferred)
    /// * `V` - The concrete type of the modal view (automatically inferred)
    /// * `F` - The callback closure type (automatically inferred)
    ///
    /// # Arguments
    /// * `modal_view` - The modal view builder
    /// * `close_callback` - An optional closure that receives a mutable reference to the parent view and a reference to the modal view with their concrete types
    ///
    /// # Example
    /// ```
    /// self.vm.show_modal(AlertView::builder("hello"), Some(|main_view: &mut MainView, alert: &AlertView| {
    ///     info!("callback = {}", main_view.lf);
    ///     main_view.lf += 1;
    ///     ManagerAction::new(false)
    /// }));
    /// ```
    pub fn show_modal<P, V, F>(&self, modal_view: ViewBuilder, close_callback: Option<F>)
    where
        P: View,
        V: View,
        F: FnOnce(&mut P, &V) -> ManagerAction + 'static,
    {
        let type_erased_callback = Box::new(
            move |view: &mut dyn View, modal: &dyn View| -> ManagerAction {
                let view_as_p: &mut P = unsafe { &mut *(view as *mut dyn View as *mut P) };
                let modal_as_v: &V = unsafe { &*(modal as *const dyn View as *const V) };
                close_callback.map_or(ManagerAction::new(false), |cb| cb(view_as_p, modal_as_v))
            },
        );

        self.show_modal_generic(modal_view, Some(type_erased_callback));
    }

    fn register_receive_views(&self, mv: Rc<RefCell<ManagedView>>) {
        if mv.borrow().publish_events {
            self.receive_events_views.borrow_mut().push(mv.clone());
            debug!(
                "register_receive_views view id='{}' unique_id='{}' will receive events",
                mv.borrow().id,
                mv.borrow().unique_id
            );
        }
        for rc_mv in mv.borrow().children.iter() {
            self.register_receive_views(rc_mv.clone());
        }
    }

    /// Adds a new view to the ViewManager.
    /// # Arguments
    /// * `id` - A unique (in the scope of the top level views) identifier for the view
    /// * `v` - The ViewBuilder used to construct the view
    /// * `active_path` - A slice of view IDs representing the path to the active view.
    ///   The active view will receive keyboard events.
    ///   The first element of the slice must be the `id`.
    pub fn add_view(&self, id: u16, v: ViewBuilder, active_path: &[usize]) {
        debug!(
            "Adding view id='{}' with active_path='{:?}'",
            id, active_path
        );
        let mut v = v.build();
        v.id = id as usize;

        // Register views that receive events
        let rc = Rc::new(RefCell::new(v));

        // Initialize the view
        self.init_view_tree(&mut rc.borrow_mut());

        // Register views to be notified
        self.register_receive_views(rc.clone());

        self.views.borrow_mut().push(rc);

        if active_path.is_empty() {
            self.active_view.borrow_mut().push(None);
            return;
        }

        let found = self.search_active_view_by_ids(self.views.borrow().as_ref(), active_path);
        self.active_view.borrow_mut().push(found);
        debug!(
            "initialized active_view id='{}' with ids={:?}, result={:?}",
            id,
            active_path,
            self.active_view.borrow()
        );
    }

    pub fn resize(&self, columns: u16, rows: u16) {
        trace!("ViewManager resize to {}x{}", columns, rows);
        let area = Rect::new(0, 0, columns, rows);
        for mv in self.views.borrow().iter() {
            let mut managed_view = mv.borrow_mut();
            self.resize_managed_view(&mut managed_view, area);
        }
        trace!("exit ViewManager resize");
    }

    fn resize_managed_view(&self, managed_view: &mut ManagedView, area: Rect) {
        managed_view.area = area;
        let cs = managed_view.view.resize(area);
        for (id, rect) in cs {
            for child in managed_view.children.iter() {
                if child.borrow().id as u16 == id {
                    self.resize_managed_view(&mut child.borrow_mut(), rect);
                }
            }
        }
    }

    pub fn draw(&self, frame: &mut ratatui::Frame) {
        trace!("ViewManager draw");
        let top_level_view_idx = *self.top_level_view_idx.borrow();
        let active_view_id = self.active_view.borrow()[top_level_view_idx]
            .as_ref()
            .and_then(|v| v.last().map(|mv| mv.borrow().unique_id.clone()));

        // Regular views can be active only if there is no modal view
        let active_view_id = if self.modal_views.borrow().is_empty() {
            active_view_id.clone()
        } else {
            None
        };

        // Draw the regular views
        {
            let views = self.views.borrow();
            let idx = *self.top_level_view_idx.borrow();
            let mut managed_view = views[idx].borrow_mut();
            trace!("drawing view {}", managed_view.id);
            managed_view.draw(frame, active_view_id, false);
        }

        // draw modal views
        let len = self.modal_views.borrow().len();
        for (index, entry) in self.modal_views.borrow().iter().enumerate() {
            let mut modal_entry = entry.borrow_mut();
            modal_entry.modal_view.area = frame.area();
            trace!(
                "drawing view id={}/unique_id={}",
                modal_entry.modal_view.id, modal_entry.modal_view.unique_id
            );
            // only the topmost modal is active
            let p = if index == len - 1 {
                Some(modal_entry.modal_view.unique_id.clone())
            } else {
                None
            };
            modal_entry.modal_view.draw(frame, p, false);
        }
        trace!("exit ViewManager draw");
    }

    fn close_modal(&self) -> bool {
        if self.modal_views.borrow().is_empty() {
            return false;
        }
        self.modal_views.borrow_mut().pop();
        true
    }

    /// Handles key events for the topmost modal view if one exists.
    ///
    /// This method processes key events in the modal context and manages the modal lifecycle,
    /// including closing modals and executing their associated callbacks.
    ///
    /// # Returns
    /// - `Some(ManagerAction)` if a modal handled the event
    /// - `None` if there are no active modals
    fn handle_modal_key_event(&self, key_event: KeyEvent) -> Option<ManagerAction> {
        // Check if there's an active modal view
        let last_modal = self.modal_views.borrow().last()?.clone();

        let mut modal_entry = last_modal.borrow_mut();
        let (_event_captured, action) = modal_entry.modal_view.view.handle_key_event(key_event);

        // If the modal should close and has a callback, execute it
        let final_action = if action.close() && modal_entry.on_close.is_some() {
            debug!("modal is closing with callback");
            let mut callback_action = ManagerAction::new(true);

            if let Some(callback) = modal_entry.on_close.take() {
                let mut parent = modal_entry.parent_view.as_ref().unwrap().borrow_mut();

                debug!("calling callback");
                let cba = callback(parent.view.as_mut(), modal_entry.modal_view.view.as_ref());
                callback_action = callback_action.with_resize(cba.resize());
            }

            ManagerAction::new(true)
                .with_close(true)
                .with_resize(callback_action.resize())
        } else {
            if action.close() {
                debug!("modal is closing without callback");
            }
            action
        };

        // Release the borrow before potentially popping
        drop(modal_entry);

        Some(final_action)
    }

    /// Handles key events for the active view hierarchy.
    ///
    /// Processes a single view's key event handling and optionally broadcasts to children.
    ///
    /// This method iterates through the active view stack in reverse order (leaf to root),
    /// allowing child views to handle events before their parents. The first view that
    /// captures the event determines the resulting action.
    ///
    /// # Returns
    /// - `Some(ManagerAction)` if a view in the hierarchy handled the event
    /// - `None` if no active views exist or none captured the event
    ///
    /// # Returns
    /// `(event_captured, merged_action)` - Whether the event was captured and the resulting action
    fn handle_active_view_key_event(&self, key_event: KeyEvent) -> Option<ManagerAction> {
        debug!("handle_active_view_key_event {:?}", key_event);
        let top_level_view_idx = *self.top_level_view_idx.borrow();
        let active_view_vec = &self.active_view.borrow()[top_level_view_idx];
        let views = active_view_vec.as_ref()?;

        let mut called_views: HashSet<String> = HashSet::new();
        let mut merged_action = ManagerAction::new(false);

        // Iterate from leaf to root, giving child views first chance to handle events
        for view in views.iter().rev() {
            let (event_captured, action) =
                self.process_view_key_event(key_event, view, &mut called_views);
            merged_action.merge(&action);

            if let EventCaptured::Yes = event_captured {
                return Some(action);
            }
        }

        Some(merged_action)
    }

    fn process_view_key_event(
        &self,
        key_event: KeyEvent,
        view: &Rc<RefCell<ManagedView>>,
        called_views: &mut HashSet<String>,
    ) -> (EventCaptured, ManagerAction) {
        let unique_id = view.borrow().unique_id.clone();

        // Skip if already processed
        if !called_views.insert(unique_id) {
            return (EventCaptured::No, ManagerAction::new(false));
        }

        self.context_view.replace(Some(view.clone()));

        let mut managed_view = view.borrow_mut();

        let (event_captured, mut merged_action) = managed_view.view.handle_key_event(key_event);
        let should_broadcast = managed_view.view.broadcast_keyboard_events();
        let children: Vec<_> = managed_view.children.to_vec();
        drop(managed_view);

        // Broadcast to children if enabled
        if should_broadcast {
            debug!("broadcast is active");
            for child in children {
                debug!(
                    "handling child id='{}', unique_id='{}'",
                    child.borrow().id,
                    child.borrow().unique_id
                );
                let (_, action) = self.process_view_key_event(key_event, &child, called_views);
                merged_action.merge(&action);
            }
            debug!("end of broadcast");
        }

        (event_captured, merged_action)
    }

    pub fn handle_key_event(&self, key_event: KeyEvent) -> ManagerAction {
        trace!("handle_key_event {:?}", key_event);

        // First, check if there's an active modal view that should handle the event
        if let Some(modal_action) = self.handle_modal_key_event(key_event) {
            return modal_action;
        }

        // Then, try to handle the event through the active view hierarchy
        if let Some(action) = self.handle_active_view_key_event(key_event) {
            return action;
        }

        trace!("exit handle_key_event");
        ManagerAction::new(false)
    }

    /// Initializes the active view based on a sequence of IDs.
    ///
    /// The IDs should form a path from root to leaf view, where each ID
    /// corresponds to a ManagedView.id in the hierarchy.
    ///
    /// # Arguments
    /// * `top_level_idx` - The index of the top-level view to set the active view for
    /// * `ids` - A slice of view IDs representing the path to the active view
    pub fn initialize_active_view(&self, top_level_idx: usize, ids: &[usize]) {
        if ids.is_empty() {
            self.active_view.borrow_mut()[top_level_idx] = None;
            return;
        }

        let found = self.search_active_view_by_ids(self.views.borrow().as_ref(), ids);
        self.active_view.borrow_mut()[top_level_idx] = found;
        debug!(
            "initialized active_view with ids={:?}, result={:?}",
            ids,
            self.active_view.borrow()
        );
    }

    fn search_active_view_by_ids(
        &self,
        views: &[Rc<RefCell<ManagedView>>],
        ids: &[usize],
    ) -> Option<Vec<Rc<RefCell<ManagedView>>>> {
        if ids.is_empty() {
            return None;
        }

        // Find the root view matching the first ID
        let mut active_view = views.iter().find(|v| v.borrow().id == ids[0]).cloned();

        let mut active_view_vec = if let Some(v) = active_view.clone() {
            vec![v]
        } else {
            return None;
        };

        // Traverse the hierarchy following the remaining IDs
        for &target_id in &ids[1..] {
            let current_view = active_view.as_ref()?;

            trace!(
                "searching for id {} in view id {}/{}",
                target_id,
                current_view.borrow().id,
                current_view.borrow().unique_id
            );

            let next_view = current_view
                .borrow()
                .children
                .iter()
                .find(|child| child.borrow().id == target_id)
                .cloned();

            if let Some(next) = next_view {
                active_view_vec.push(next.clone());
                active_view = Some(next);
            } else {
                warn!("id {} not found in children", target_id);
                return None;
            }
        }

        Some(active_view_vec)
    }

    pub fn handle_mouse_event(&self, mouse_event: MouseEvent) -> ManagerAction {
        //trace!("handle_mouse_event {:?}", mouse_event);

        if !matches!(mouse_event.kind, crossterm::event::MouseEventKind::Down(_)) {
            return ManagerAction::new(false);
        }

        // on mouse down, activate the view at the mouse position
        let position = Position::new(mouse_event.column, mouse_event.row);
        self.activate_view_at_position(position);

        // let's notify the views
        let ma = self.handle_active_view_mouse_event(mouse_event);

        ma.unwrap_or_else(|| ManagerAction::new(false))
    }

    fn activate_view_at_position(&self, position: Position) {
        let found = self.search_active_view(position);
        let top_level_view_idx = *self.top_level_view_idx.borrow();
        self.active_view.borrow_mut()[top_level_view_idx] = found;
        trace!("active_view={:?}", self.active_view.borrow());
    }

    /// Handles mouse events for the active view hierarchy.
    ///
    /// This method iterates through the active view stack in reverse order (leaf to root),
    /// allowing child views to handle events before their parents.
    ///
    /// # Returns
    /// - `Some(ManagerAction)` if a view in the hierarchy handled the event
    /// - `None` if no active views exist
    fn handle_active_view_mouse_event(&self, mouse_event: MouseEvent) -> Option<ManagerAction> {
        debug!("handle_active_view_mouse_event {:?}", mouse_event);
        let top_level_view_idx = *self.top_level_view_idx.borrow();
        let active_view_vec = &self.active_view.borrow()[top_level_view_idx];
        let views = active_view_vec.as_ref()?;

        let mut called_views: HashSet<String> = HashSet::new();
        let mut merged_action = ManagerAction::new(false);

        // Iterate from leaf to root, giving child views first chance to handle events
        for view in views.iter().rev() {
            let action = self.process_view_mouse_event(mouse_event, view, &mut called_views);
            merged_action.merge(&action);
        }

        Some(merged_action)
    }

    fn process_view_mouse_event(
        &self,
        mouse_event: MouseEvent,
        view: &Rc<RefCell<ManagedView>>,
        called_views: &mut HashSet<String>,
    ) -> ManagerAction {
        let unique_id = view.borrow().unique_id.clone();

        // Skip if already processed
        if !called_views.insert(unique_id) {
            return ManagerAction::new(false);
        }

        self.context_view.replace(Some(view.clone()));

        let mut managed_view = view.borrow_mut();
        let area = managed_view.area;

        let mut merged_action = managed_view.view.handle_mouse_event(area, mouse_event);
        let should_broadcast = managed_view.view.broadcast_keyboard_events();
        let children: Vec<_> = managed_view.children.to_vec();
        drop(managed_view);

        // Broadcast to children if enabled
        if should_broadcast {
            debug!("broadcast is active for mouse event");
            for child in children {
                debug!(
                    "handling child id='{}', unique_id='{}'",
                    child.borrow().id,
                    child.borrow().unique_id
                );
                let action = self.process_view_mouse_event(mouse_event, &child, called_views);
                merged_action.merge(&action);
            }
            debug!("end of broadcast for mouse event");
        }

        merged_action
    }

    fn search_active_view(&self, position: Position) -> Option<Vec<Rc<RefCell<ManagedView>>>> {
        let idx = *self.top_level_view_idx.borrow();

        let views = self.views.borrow();
        let mut active_view = views[idx].clone();

        let mut active_view_vec = vec![active_view.clone()];

        loop {
            let current_view = active_view;
            trace!(
                "view id {}/{} contains position {:?}",
                current_view.borrow().id,
                current_view.borrow().unique_id,
                position
            );
            if current_view.borrow().view.capture_focus() {
                return Some(active_view_vec);
            }

            let next_view = current_view
                .borrow()
                .children
                .iter()
                .find(|child| child.borrow().area.contains(position))
                .cloned();

            if let Some(next) = next_view {
                active_view_vec.push(next.clone());
                active_view = next.clone();
            } else {
                return Some(active_view_vec);
            }
        }
    }

    pub fn handle_broadcast_event(&self, event: &Result<GenericEvent, RecvError>) -> ManagerAction {
        let mut manager_action: ManagerAction = ManagerAction::new(false);
        match event {
            Ok(ge) => match ge {
                GenericEvent::ViewManagerEvent(vme) => match vme {
                    ViewManagerEvent::Resize => {
                        manager_action.resize = true;
                    }
                    ViewManagerEvent::Redraw => {
                        manager_action.redraw = true;
                    }
                    ViewManagerEvent::Exit(payload) => {
                        self.exit_string.replace(payload.clone());
                        manager_action.close = true;
                    }
                },
                GenericEvent::ApplicationEvent(ae) => {
                    debug!("received application event: '{}'", ae.id);
                    for rv in self.receive_events_views.borrow().iter() {
                        debug!(
                            "notifying view id='{}', unique_id='{}'",
                            rv.borrow().id,
                            rv.borrow().unique_id
                        );
                        rv.borrow_mut().view.handle_application_event(ae);
                    }
                }
            },
            Err(e) => {
                warn!("broadcast recv error: {:?}", e);
            }
        };
        manager_action
    }

    pub fn handle_crossterm_event(
        &self,
        crossterm_event: Option<std::io::Result<Event>>,
    ) -> ManagerAction {
        debug!("received crossterm event: {:?}", crossterm_event);
        let mut manager_action: ManagerAction = ManagerAction::new(false);
        match crossterm_event {
            Some(Ok(event)) => match event {
                Event::Resize(columns, rows) => {
                    // when receiving a resize event, it is needed to handle it here with
                    // the given columns and rows, because the terminal size has not yet
                    // been updated.
                    debug!("received resize event: col='{}' row='{}'", columns, rows);
                    self.resize(columns, rows);
                    manager_action.redraw = true;
                    manager_action.resize = false; // prevent the main loop to wrongly apply a resize
                }
                Event::Mouse(mouse_event) => {
                    manager_action = self.handle_mouse_event(mouse_event);
                }
                Event::Key(key_event) => match key_event.code {
                    KeyCode::Esc => {
                        manager_action.close = true;
                    }
                    _ => {
                        if key_event.code == KeyCode::Tab && self.modal_views.borrow().is_empty() {
                            self.switch_to_next_top_level_view();
                            manager_action.redraw = true;
                        } else if key_event.modifiers.contains(KeyModifiers::CONTROL)
                            && let KeyCode::Char('h') = key_event.code
                            && let Some(global_help_view_builder_cb) =
                                &self.global_help_view_builder_cb
                        {
                            self.show_modal_generic(global_help_view_builder_cb(), None);
                            manager_action.redraw = true;
                        } else {
                            manager_action = self.handle_key_event(key_event);
                        }
                    }
                },
                _ => {
                    manager_action.redraw = true;
                }
            },
            Some(Err(e)) => println!("Error: {e:?}\r"),
            None => {}
        };
        manager_action
    }

    fn switch_to_next_top_level_view(&self) {
        let tlvi = self.top_level_view_idx.borrow();
        let idx = tlvi.add(1) % self.views.borrow().len();
        drop(tlvi);
        debug!("switching to top level view idx={}", idx);
        self.top_level_view_idx.replace(idx);
    }

    pub async fn event_loop(&self) -> Option<String> {
        let mut stdout = std::io::stdout();
        crossterm::execute!(stdout, EnableMouseCapture).expect("failed to enable mouse capture");

        let mut term = ratatui::init();
        let init_rect = term.get_frame().area();
        self.resize(init_rect.width, init_rect.height);

        // initial draw
        let _ = term.draw(|frame| {
            self.draw(frame);
        });

        let mut crossterm_reader = EventStream::new();
        let mut rx = self.tx.subscribe();

        let mut manager_action: ManagerAction = ManagerAction::new(false);
        while !manager_action.close {
            let crossterm_event_next = crossterm_reader.next();
            select! {
                broadcast_event = rx.recv() => {
                    manager_action = self.handle_broadcast_event(&broadcast_event);
                }
                crossterm_event = crossterm_event_next => {
                    manager_action = self.handle_crossterm_event(crossterm_event);
                }
            }

            if manager_action.close {
                info!("ViewManager received close signal");
                // close the topmost modal if any, else close the application
                if self.close_modal() {
                    manager_action.close = false;
                    manager_action.redraw = true;
                }
            }
            if manager_action.resize() {
                debug!("ViewManager resizing");
                let init_rect = term.get_frame().area();
                self.resize(init_rect.width, init_rect.height);
                manager_action.redraw = true;
            }
            if manager_action.redraw() {
                debug!("ViewManager redrawing");
                let _ = term.draw(|frame| {
                    self.draw(frame);
                });
            }
        }
        let mut stdout = std::io::stdout();
        crossterm::execute!(stdout, DisableMouseCapture).expect("failed to disable mouse capture");
        ratatui::restore();

        self.exit_string.take()
    }
}
