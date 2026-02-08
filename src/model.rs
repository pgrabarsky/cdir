use std::sync::Arc;

use log::{debug, error, trace};
use tokio::sync::broadcast;

use crate::tui::{GenericEvent, event::ApplicationEvent};

#[cfg(test)]
#[path = "model_tests.rs"]
mod model_tests;

// "data.payload"
pub struct DataStatePayload {
    pub objects_type: String,
    pub is_empty: bool,
}

/// A type alias for a function that retrieves a list of data entries based on the given parameters.
///
/// # Type Parameters
/// - `'store`: The lifetime of the data store or context from which the data is retrieved.
/// - `T`: The type of the data entries being retrieved.
///
/// # Parameters
/// - `start`: The starting index of the data entries to retrieve.
/// - `count`: The maximum number of data entries to retrieve.
/// - `filter`: A string used as a filter or search term for the data entries.
/// - `fuzzy`: If true, perform a fuzzy search ; else perform an exact search
///
/// # Returns
/// - `Result<Vec<T>, rusqlite::Error>`: A `Result` containing either a vector of data entries
///   (`Vec<T>`) on success or a `rusqlite::Error` on failure.
pub(crate) type ListFunction<T> =
    dyn Fn(usize, usize, &str, bool) -> Result<Vec<T>, rusqlite::Error>;

/// A model representing a view of data, typically used for managing and displaying
/// a subset of entries with filtering and pagination capabilities.
///
/// # Type Parameters
/// - `'store`: A lifetime parameter associated with the data store.
/// - `T`: The type of the entries being managed.
///
/// # Fields
/// - `entries`: An optional vector containing the entries of type `T` to be displayed.
/// - `list_fn`: A boxed function or closure responsible for fetching or generating the list of entries.
/// - `first`: The index of the first entry in the current view.
/// - `length`: The number of entries to display in the current view.
/// - `filter`: A string used to filter the entries based on some criteria.
pub(crate) struct DataViewModel<T> {
    objects_type: String,
    tx: broadcast::Sender<GenericEvent>,
    pub(crate) entries: Option<Vec<T>>,
    pub(crate) list_fn: Box<ListFunction<T>>,
    pub(crate) first: usize,
    pub(crate) length: u16,
    filter: String,
    fuzzy_match: bool,
}

impl<T: Clone> DataViewModel<T> {
    /// Creates a new instance of `DataViewModel`.
    ///
    /// ### Parameters
    /// - `list_fn`: A boxed function that retrieves a list of data entries based on
    ///   the specified range and filter text.
    ///
    /// ### Returns
    /// A new `DataViewModel` instance.    
    pub(crate) fn new(
        objects_type: String,
        tx: broadcast::Sender<GenericEvent>,
        list_fn: Box<ListFunction<T>>,
        fuzzy_match: bool,
    ) -> Self {
        DataViewModel {
            objects_type,
            tx,
            entries: Option::None,
            list_fn,
            first: 0,
            length: 0,
            filter: String::new(),
            fuzzy_match,
        }
    }

    fn publish(&self) {
        let event = GenericEvent::ApplicationEvent(ApplicationEvent {
            id: String::from("data.payload"),
            payload: Some(Arc::new(DataStatePayload {
                objects_type: self.objects_type.clone(),
                is_empty: self.length == 0,
            })),
        });
        debug!("model sending event={:?}", event);
        let result = self.tx.send(event);
        if let Err(e) = result {
            error!("Failed to send 'data.payload' event: {}", e);
        }
    }

    /// Checks if the current data view is a subset of the specified range and filter.
    /// ### Parameters
    /// - `first`: The starting index of the range.
    /// - `length`: The length of the range.
    /// - `text`: The filter text.
    ///
    /// ### Returns
    /// `true` if the current data view is a subset of the specified range and filter;
    /// otherwise, `false`.
    fn is_a_subset_of(&mut self, first: usize, length: u16) -> bool {
        self.entries.is_some()
            && (first >= self.first)
            && (first + length as usize <= self.first + self.length as usize)
    }

    /// Updates the current data view to a subset of the specified range and filter,
    /// if possible.
    ///
    /// ### Parameters
    /// - `first`: The starting index of the range.
    /// - `length`: The length of the range.
    /// - `text`: The filter text.
    ///
    /// ### Returns
    /// `true` if the update was successful; otherwise, `false`.
    fn update_into_subset(&mut self, first: usize, length: u16) -> bool {
        if !self.is_a_subset_of(first, length) {
            return false;
        }
        if let Some(self_entries) = &self.entries {
            let offset = self.first - first;
            self.entries = Some(self_entries[offset..(length as usize)].to_vec());
        }
        self.first = first;
        self.length = length;
        self.publish();
        true
    }

    pub(crate) fn set_fuzzy_match(&mut self, fuzzy_match: bool) {
        debug!("fuzzy_match={}", fuzzy_match);
        if self.fuzzy_match == fuzzy_match {
            return;
        }
        self.fuzzy_match = fuzzy_match;
        self.update(self.first, self.length, true);
    }

    pub(crate) fn update_filter(&mut self, length: u16, filter: &str, fuzzy: bool) {
        self.filter = String::from(filter);
        self.fuzzy_match = fuzzy;
        self.update(0, length, true);
    }

    /// Updates the data view with new entries based on the specified range and filter.
    /// If the requested range is already a subset of the current data, no update occurs.
    ///
    /// If the range [first, first + length] exceeds the available data or if the result
    /// is a subset of the current view, the update is not performed.
    ///
    /// ### Parameters
    /// - `first`: The starting index of the range.
    /// - `length`: The length of the range.
    /// - `text`: The filter text.
    /// - `force`: A boolean indicating whether to force the update even if no data is found (if not a subset of the current view).
    ///
    /// ### Returns
    /// `true` if the data view was updated; otherwise, `false`.
    pub(crate) fn update(&mut self, first: usize, length: u16, force: bool) -> bool {
        trace!("update first={} length={} force={}", first, length, force);
        if !force && !self.fuzzy_match && self.update_into_subset(first, length) {
            trace!("subset found");
            return false;
        }
        let new_entries: Result<Vec<T>, rusqlite::Error> =
            (self.list_fn)(first, length as usize, &self.filter, self.fuzzy_match);
        match new_entries {
            Ok(new_entries) => {
                let new_length = new_entries.len();
                if !force && (new_length != length as usize) {
                    // If we have less data than requested and it is a subset, we don't update
                    // This is the case for a scroll out of the data.
                    if self.is_a_subset_of(first, new_length as u16) {
                        trace!("Data is a subset, no update");
                        return false;
                    }
                }
                if new_length > 0 {
                    self.entries = Some(new_entries);
                    self.first = first;
                    self.length = new_length as u16;
                    trace!("Updated length={}", self.length);
                    self.publish();

                    true
                } else {
                    debug!("No data found");
                    if force {
                        self.entries = Option::None;
                        self.first = 0;
                        self.length = 0;
                        trace!("Forced update length={}", self.length);
                        self.publish();
                        return true;
                    }
                    false
                }
            }
            Err(err) => {
                error!("update_data_pl: {}", err);
                false
            }
        }
    }

    /// Updates the data view by applying an offset to the current starting index and
    /// adjusting the range accordingly.
    ///
    /// ### Parameters
    /// - `offset`: The offset to apply to the current starting index.
    /// - `length`: The length of the range to view.
    /// - `text`: The filter text.
    ///
    /// ### Returns
    /// `true` if the data view was updated; otherwise, `false`.
    pub(crate) fn update_to_offset(&mut self, offset: i64, length: u16) -> bool {
        let first: usize = if self.first as i64 + offset < 0 {
            0
        } else {
            (self.first as i64 + offset) as usize
        };
        trace!("update_data_pos self.first={} first={}", self.first, first);
        self.update(first, length, false)
    }

    /// Reloads the current data view by fetching new entries based on the existing
    /// starting index, length, and filter.
    pub(crate) fn reload(&mut self) {
        let new_entries: Result<Vec<T>, rusqlite::Error> = (self.list_fn)(
            self.first,
            self.length as usize,
            self.filter.as_str(),
            self.fuzzy_match,
        );
        match new_entries {
            Ok(new_entries) => {
                let new_length = new_entries.len();
                if new_length > 0 {
                    self.entries = Some(new_entries);
                    self.length = new_length as u16;
                    trace!("Updated");
                    self.publish();
                } else {
                    debug!("No data found");
                    self.entries = Option::None;
                    self.length = 0;
                    self.publish();
                }
            }
            Err(err) => {
                debug!("No data found {}", err);
            }
        }
    }
}
