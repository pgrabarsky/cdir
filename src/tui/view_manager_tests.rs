use ratatui::layout::Rect;

use super::*;
use crate::tui::{ViewBuilder, view::View};

struct TestView {}
impl TestView {
    fn builder() -> ViewBuilder { ViewBuilder::from(Box::new(TestView {})) }
}
impl View for TestView {
    fn draw(&mut self, _: &mut ratatui::Frame, _: Rect, _: bool) {}
}

struct LeafView {}
impl LeafView {
    fn builder() -> ViewBuilder { ViewBuilder::from(Box::new(LeafView {})) }
}
impl View for LeafView {
    fn draw(&mut self, _: &mut ratatui::Frame, _: Rect, _: bool) {}
}

struct RootView {}
impl RootView {
    fn builder() -> ViewBuilder {
        ViewBuilder::from(Box::new(RootView {}))
            .child(0, LeafView::builder())
            .child(1, LeafView::builder())
    }
}
impl View for RootView {
    fn draw(&mut self, _: &mut ratatui::Frame, _: Rect, _: bool) {}
}

#[test]
fn test_build_leaf() {
    let vb = LeafView::builder();
    let mut managed_view = vb.build();

    assert!(managed_view.children.is_empty());
    let _ = managed_view.view.resize(Rect::new(0, 0, 0, 0));
}

#[test]
fn test_build_root() {
    let vb = RootView::builder();
    let mut managed_view = vb.build();
    assert_eq!(2, managed_view.children.len());
    let _ = managed_view.view.resize(Rect::new(0, 0, 0, 0));
}

#[test]
fn test_add() {
    let vm = ViewManager::new();
    vm.add_view(0, TestView::builder(), &[]);
    assert!(vm.views.borrow().len() == 1);
    vm.add_view(1, TestView::builder(), &[]);
    assert!(vm.views.borrow().len() == 2);
}

#[test]
fn test_active() {
    let vm = ViewManager::new();
    let v1 = TestView::builder();
    let v2 = TestView::builder();

    vm.add_view(0, v1, &[]);
    vm.add_view(1, v2, &[]);
    assert!(vm.views.borrow().len() == 2);

    let id1 = vm.views.borrow()[0].borrow().id;
    let id2 = vm.views.borrow()[1].borrow().id;
    assert_ne!(id1, id2);
}

#[test]
fn test_initialize_active_view() {
    let vm = ViewManager::new();

    // Add a root view with children
    vm.add_view(10, RootView::builder(), &[]);

    // Initially no active view
    assert!(vm.active_view.borrow()[0].is_none());

    // Set active view to the root (id=10)
    vm.initialize_active_view(0, &[10]);
    assert!(vm.active_view.borrow()[0].is_some());
    assert_eq!(vm.active_view.borrow()[0].as_ref().unwrap().len(), 1);
    assert_eq!(
        vm.active_view.borrow()[0].as_ref().unwrap()[0].borrow().id,
        10
    );

    // Set active view to a child (path: root=10 -> child=0)
    vm.initialize_active_view(0, &[10, 0]);
    assert!(vm.active_view.borrow()[0].is_some());
    assert_eq!(vm.active_view.borrow()[0].as_ref().unwrap().len(), 2);
    assert_eq!(
        vm.active_view.borrow()[0].as_ref().unwrap()[0].borrow().id,
        10
    );
    assert_eq!(
        vm.active_view.borrow()[0].as_ref().unwrap()[1].borrow().id,
        0
    );

    // Set active view to another child (path: root=10 -> child=1)
    vm.initialize_active_view(0, &[10, 1]);
    assert!(vm.active_view.borrow()[0].is_some());
    assert_eq!(vm.active_view.borrow()[0].as_ref().unwrap().len(), 2);
    assert_eq!(
        vm.active_view.borrow()[0].as_ref().unwrap()[1].borrow().id,
        1
    );

    // Invalid path should result in None
    vm.initialize_active_view(0, &[10, 99]);
    assert!(vm.active_view.borrow()[0].is_none());

    // Empty IDs should clear active view
    vm.initialize_active_view(0, &[]);
    assert!(vm.active_view.borrow()[0].is_none());
}

#[test]
fn test_centered_rect() {
    // Test centering in a 100x50 area
    let area = Rect::new(0, 0, 100, 50);
    let centered = ViewManager::centered_rect(area, 20, 10);

    // Should be centered at (40, 20) with size 20x10
    assert_eq!(centered.x, 40);
    assert_eq!(centered.y, 20);
    assert_eq!(centered.width, 20);
    assert_eq!(centered.height, 10);

    // Test with offset area
    let area = Rect::new(10, 5, 100, 50);
    let centered = ViewManager::centered_rect(area, 20, 10);

    // Should be centered at (10 + 40, 5 + 20) with size 20x10
    assert_eq!(centered.x, 50);
    assert_eq!(centered.y, 25);
    assert_eq!(centered.width, 20);
    assert_eq!(centered.height, 10);

    // Test clamping when requested size is larger than area
    let area = Rect::new(0, 0, 50, 30);
    let centered = ViewManager::centered_rect(area, 100, 50);

    // Should be clamped to area size and positioned at origin
    assert_eq!(centered.x, 0);
    assert_eq!(centered.y, 0);
    assert_eq!(centered.width, 50);
    assert_eq!(centered.height, 30);

    // Test exact fit
    let area = Rect::new(0, 0, 50, 50);
    let centered = ViewManager::centered_rect(area, 50, 50);

    assert_eq!(centered.x, 0);
    assert_eq!(centered.y, 0);
    assert_eq!(centered.width, 50);
    assert_eq!(centered.height, 50);
}
