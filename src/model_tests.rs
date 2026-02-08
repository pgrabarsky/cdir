use tokio::sync::broadcast;

use crate::{model::DataViewModel, store::Store, tui::GenericEvent};

#[test]
fn test_scroll() {
    let tx = broadcast::channel::<GenericEvent>(16).0;
    let store = Store::setup_test_store();
    store.add_path("/5").unwrap();
    store.add_path("/4").unwrap();
    store.add_path("/3").unwrap();
    store.add_path("/2").unwrap();
    store.add_path("/1").unwrap();

    let mut model = DataViewModel::new(
        "test".to_string(),
        tx,
        Box::new(move |pos, len, text, fuzzy| store.list_paths(pos, len, text, fuzzy)),
        false,
    );
    assert!(model.entries.is_none());

    model.update(0, 2, false);
    assert_eq!(model.first, 0);
    assert_eq!(model.entries.as_ref().unwrap().len(), 2);
    assert_eq!(model.entries.as_ref().unwrap()[0].path, "/1");
    assert_eq!(model.entries.as_ref().unwrap()[1].path, "/2");

    model.update(1, 2, false);
    assert_eq!(model.first, 1);
    assert_eq!(model.entries.as_ref().unwrap().len(), 2);
    assert_eq!(model.entries.as_ref().unwrap()[0].path, "/2");
    assert_eq!(model.entries.as_ref().unwrap()[1].path, "/3");

    model.update(2, 2, false);
    assert_eq!(model.first, 2);
    assert_eq!(model.entries.as_ref().unwrap().len(), 2);
    assert_eq!(model.entries.as_ref().unwrap()[0].path, "/3");
    assert_eq!(model.entries.as_ref().unwrap()[1].path, "/4");

    model.update(3, 2, false);
    assert_eq!(model.first, 3);
    assert_eq!(model.entries.as_ref().unwrap().len(), 2);
    assert_eq!(model.entries.as_ref().unwrap()[0].path, "/4");
    assert_eq!(model.entries.as_ref().unwrap()[1].path, "/5");

    // The model won't update as it would only remain ["/5"] which is a subset of the current view
    model.update(4, 2, false);
    assert_eq!(model.first, 3);
    assert_eq!(model.entries.as_ref().unwrap().len(), 2);
    assert_eq!(model.entries.as_ref().unwrap()[0].path, "/4");
    assert_eq!(model.entries.as_ref().unwrap()[1].path, "/5");

    // The model won't update as it would only remain []
    model.update(5, 2, false);
    assert_eq!(model.entries.as_ref().unwrap().len(), 2);
    assert_eq!(model.entries.as_ref().unwrap()[0].path, "/4");
    assert_eq!(model.entries.as_ref().unwrap()[1].path, "/5");

    // Scroll back to 2
    model.update(2, 2, false);
    assert_eq!(model.first, 2);
    assert_eq!(model.entries.as_ref().unwrap().len(), 2);
    assert_eq!(model.entries.as_ref().unwrap()[0].path, "/3");
    assert_eq!(model.entries.as_ref().unwrap()[1].path, "/4");

    // The model will update as ["/5"] is not a subset of the current view
    model.update(4, 2, false);
    assert_eq!(model.first, 4);
    assert_eq!(model.entries.as_ref().unwrap().len(), 1);
    assert_eq!(model.entries.as_ref().unwrap()[0].path, "/5");

    // The model won't update as it would only remain []
    model.update(5, 2, false);
    assert_eq!(model.first, 4);
    assert_eq!(model.entries.as_ref().unwrap().len(), 1);
    assert_eq!(model.entries.as_ref().unwrap()[0].path, "/5");
}
