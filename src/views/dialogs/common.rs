use crate::app::Message;
use crate::fl;
use cosmic::{
    Element,
    widget::{button, dialog},
};
use std::borrow::Cow;

pub fn confirmation<'a>(
    title: impl Into<Cow<'a, str>>,
    prompt: impl Into<Cow<'a, str>>,
    ok_message: Message,
    cancel_message: Option<Message>,
    running: bool,
) -> Element<'a, Message> {
    let mut dialog = dialog::dialog().title(title).body(prompt);

    let mut ok_button = button::destructive(fl!("ok"));
    if !running {
        ok_button = ok_button.on_press(ok_message);
    }

    dialog = dialog.primary_action(ok_button);

    if let Some(c) = cancel_message {
        dialog = dialog.secondary_action(button::standard(fl!("cancel")).on_press(c))
    };

    if running {
        dialog = dialog.body(fl!("working"));
    }

    dialog.into()
}

pub fn info<'a>(
    title: impl Into<Cow<'a, str>>,
    body: impl Into<Cow<'a, str>>,
    ok_message: Message,
) -> Element<'a, Message> {
    dialog::dialog()
        .title(title)
        .body(body)
        .primary_action(button::standard(fl!("ok")).on_press(ok_message))
        .into()
}
