use crate::app::Message;
use crate::controls::wizard::{wizard_action_row, wizard_shell};
use crate::fl;
use crate::message::dialogs::{FormatDiskMessage, SmartDialogMessage};
use crate::state::dialogs::{FormatDiskDialog, SmartDataDialog};
use cosmic::iced::widget as iced_widget;
use cosmic::{
    Element,
    widget::text::{caption, caption_heading},
    widget::{button, dialog, dropdown},
};

pub fn format_disk<'a>(state: FormatDiskDialog) -> Element<'a, Message> {
    let erase_options = vec![
        fl!("erase-dont-overwrite-quick").to_string(),
        fl!("erase-overwrite-slow").to_string(),
    ];

    let partitioning_options = vec![
        fl!("partitioning-dos-mbr").to_string(),
        fl!("partitioning-gpt").to_string(),
        fl!("partitioning-none").to_string(),
    ];

    let mut content = iced_widget::column![
        caption_heading(fl!("erase")),
        dropdown(erase_options, Some(state.erase_index), |v| {
            FormatDiskMessage::EraseUpdate(v).into()
        }),
        caption_heading(fl!("partitioning")),
        dropdown(partitioning_options, Some(state.partitioning_index), |v| {
            FormatDiskMessage::PartitioningUpdate(v).into()
        }),
    ]
    .spacing(12);

    if state.running {
        content = content.push(caption(fl!("working")));
    }

    let mut confirm = button::destructive(fl!("format-disk"));
    if !state.running {
        confirm = confirm.on_press(FormatDiskMessage::Confirm.into());
    }

    let footer = wizard_action_row(
        vec![],
        vec![
            button::standard(fl!("cancel"))
                .on_press(FormatDiskMessage::Cancel.into())
                .into(),
            confirm.into(),
        ],
    );

    wizard_shell(caption(fl!("format-disk")).into(), content.into(), footer)
}

pub fn smart_data<'a>(state: SmartDataDialog) -> Element<'a, Message> {
    let mut content = iced_widget::column![]
        .spacing(6)
        .width(cosmic::iced::Length::Fill);

    if let Some(err) = state.error.as_ref() {
        content = content.push(caption(err.clone()));
    }

    if let Some((status, attributes)) = state.info.as_ref() {
        content = content
            .push(caption_heading(fl!("smart-data-self-tests")))
            .push(caption(format!("{}: {}", fl!("smart-type"), status.device)))
            .push(caption(format!(
                "{}: {}",
                fl!("smart-updated"),
                fl!("unknown")
            )));

        if let Some(temp_c) = status.temperature_celsius {
            content = content.push(caption(format!(
                "{}: {} °C",
                fl!("smart-temperature"),
                temp_c
            )));
        }

        if let Some(hours) = status.power_on_hours {
            content = content.push(caption(format!("{}: {hours}", fl!("smart-power-on-hours"))));
        }

        let selftest_str = if status.test_running {
            status
                .test_percent_remaining
                .map(|p| format!("Running ({}% remaining)", p))
                .unwrap_or_else(|| "Running".to_string())
        } else {
            "Idle".to_string()
        };
        content = content.push(caption(format!(
            "{}: {}",
            fl!("smart-selftest"),
            selftest_str
        )));

        if !attributes.is_empty() {
            content = content.push(caption_heading(fl!("details")));
            for attr in attributes {
                content = content.push(caption(format!(
                    "{}: {} (raw: {})",
                    attr.name, attr.current, attr.raw_value
                )));
            }
        }
    } else if state.running {
        content = content.push(caption(fl!("working")));
    } else {
        content = content.push(caption(fl!("smart-no-data")));
    }

    let mut refresh = button::standard(fl!("refresh"));
    let mut short = button::standard(fl!("smart-selftest-short"));
    let mut extended = button::standard(fl!("smart-selftest-extended"));
    let mut abort = button::standard(fl!("smart-selftest-abort"));
    let mut close = button::standard(fl!("close"));

    if !state.running {
        refresh = refresh.on_press(SmartDialogMessage::Refresh.into());
        short = short.on_press(SmartDialogMessage::SelfTestShort.into());
        extended = extended.on_press(SmartDialogMessage::SelfTestExtended.into());
        abort = abort.on_press(SmartDialogMessage::AbortSelfTest.into());
        close = close.on_press(SmartDialogMessage::Close.into());
    }

    let controls = iced_widget::column![
        iced_widget::row![refresh, short].spacing(8),
        iced_widget::row![extended, abort].spacing(8),
    ]
    .spacing(8);

    dialog::dialog()
        .title(fl!("smart-data-self-tests"))
        .control(content.push(controls))
        .primary_action(close)
        .into()
}
