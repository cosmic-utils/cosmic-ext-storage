use super::wizard_step_is_clickable;

#[test]
fn breadcrumb_previous_step_is_clickable() {
    assert!(wizard_step_is_clickable(1, 3));
}

#[test]
fn breadcrumb_current_step_is_not_clickable() {
    assert!(!wizard_step_is_clickable(2, 2));
}

#[test]
fn breadcrumb_future_step_is_not_clickable() {
    assert!(!wizard_step_is_clickable(3, 2));
}
