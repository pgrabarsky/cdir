use std::any::Any;

use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::layout::Rect;

use crate::tui::event::ApplicationEvent;

/// Represents actions that the ViewManager should take after handling an event.
#[derive(Debug, Clone, Copy)]
pub struct ManagerAction {
    pub(crate) redraw: bool,
    pub(crate) resize: bool,
    pub(crate) close: bool,
}

impl ManagerAction {
    /// Creates a new ManagerAction with the specified redraw flag.
    pub fn new(redraw: bool) -> ManagerAction {
        ManagerAction {
            redraw,
            resize: false,
            close: false,
        }
    }

    pub fn merge(&mut self, other: &ManagerAction) {
        self.redraw |= other.redraw;
        self.resize |= other.resize;
        self.close |= other.close;
    }

    /// Marks that a resize operation is needed.
    pub fn with_resize(mut self, resize: bool) -> ManagerAction {
        self.resize = resize;
        self
    }

    /// Marks that the view should be closed.
    pub fn with_close(mut self, close: bool) -> ManagerAction {
        self.close = close;
        self
    }

    /// Returns whether a redraw is needed.
    pub fn redraw(&self) -> bool { self.redraw }

    /// Returns whether a resize is needed.
    pub fn resize(&self) -> bool { self.resize }

    /// Returns whether the view should be closed.
    pub fn close(&self) -> bool { self.close }
}

/// Indicates whether an event was captured and handled by a view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventCaptured {
    /// The event was captured and handled by the view.
    Yes,
    /// The event was not handled and should propagate.
    No,
}

/// A trait representing a view in the TUI system.
///
/// Views are responsible for rendering themselves and handling input events.
/// They can have child views and participate in a view hierarchy.
#[allow(unused)]
pub trait View: Any {
    fn init(&mut self) {}

    /// Capturing the focus will prevent sub-views to get the focus that will be given to the capturing view.
    fn capture_focus(&self) -> bool { false }

    /// When set, the keyboard events sent to the view will also be sent to all sub views.
    fn broadcast_keyboard_events(&self) -> bool { false }

    /// Called when the view's area changes.
    ///
    /// Returns a list of (child_id, rect) tuples indicating how child views
    /// should be positioned within this view's area.
    ///
    /// Default implementation returns an empty vector (no children).
    fn resize(&mut self, area: Rect) -> Vec<(u16, Rect)> {
        let _ = area;
        vec![]
    }

    /// Renders the view to the given frame.
    ///
    /// # Arguments
    /// * `frame` - The ratatui frame to render to
    /// * `area` - The rectangular area this view should occupy
    /// * `active` - Whether this view is currently the active/focused view
    fn draw(&mut self, frame: &mut ratatui::Frame, area: Rect, active: bool);

    fn handle_application_event(&mut self, _ae: &ApplicationEvent) {}

    /// Handles a keyboard event.
    ///
    /// Returns a tuple of (EventCaptured, ManagerAction):
    /// - EventCaptured::Yes stops event propagation to parent views
    /// - EventCaptured::No allows the event to bubble up
    /// - ManagerAction specifies what the ViewManager should do next
    ///
    /// Default implementation does not capture events.
    fn handle_key_event(&mut self, key_event: KeyEvent) -> (EventCaptured, ManagerAction) {
        let _ = key_event;
        (EventCaptured::No, ManagerAction::new(false))
    }

    /// Handles a mouse event.
    ///
    /// Returns a ManagerAction specifying what the ViewManager should do next.
    ///
    /// Default implementation takes no action.
    fn handle_mouse_event(&mut self, mouse_event: MouseEvent) -> ManagerAction {
        let _ = mouse_event;
        ManagerAction::new(false)
    }
}
