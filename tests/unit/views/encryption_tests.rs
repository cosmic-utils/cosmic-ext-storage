use super::*;
use cosmic::iced::{
    Size,
    advanced::{Layout, Widget, layout, mouse, widget::Tree},
};
use rstest::rstest;

#[rstest]
#[case::hidden(false)]
#[case::revealed(true)]
fn encryption_passphrase_stays_protected_when_revealed(#[case] reveal: bool) {
    let secret = "fixture-secret-é🦀";
    let input = encryption_passphrase_input(secret.into(), reveal);
    let tree = Tree::new(&input as &dyn Widget<Message, cosmic::Theme, cosmic::Renderer>);
    // Label, input row (text and leading lock icon), matching the real widget's
    // layout hierarchy. No display server or native storage is involved.
    let layout = layout::Node::with_children(
        Size::new(240.0, 64.0),
        vec![
            layout::Node::new(Size::new(240.0, 24.0)),
            layout::Node::with_children(
                Size::new(240.0, 40.0),
                vec![
                    layout::Node::new(Size::new(200.0, 40.0)),
                    layout::Node::with_children(
                        Size::new(40.0, 40.0),
                        vec![layout::Node::new(Size::new(16.0, 16.0))],
                    ),
                ],
            ),
        ],
    );
    let nodes = input.a11y_nodes(Layout::new(&layout), &tree, mouse::Cursor::Unavailable);
    let field = nodes.root()[0].node();
    assert_eq!(format!("{:?}", field.role()), "PasswordInput");
    assert!(!field.value().unwrap().is_empty());
    assert!(
        field
            .value()
            .unwrap()
            .chars()
            .all(|character| character == '•')
    );
    assert!(!format!("{nodes:?}").contains(secret));
}
