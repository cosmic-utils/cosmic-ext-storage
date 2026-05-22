use cosmic::{Element, cosmic_theme, iced::Alignment, iced::Length, theme, widget};
use storage_types::FilesystemToolInfo;

use crate::{
    app::{Message, REPOSITORY},
    config::Config,
    fl,
};

pub fn settings<'a>(config: &Config) -> Element<'a, Message> {
    let cosmic_theme::Spacing {
        space_s, space_m, ..
    } = theme::active().cosmic().spacing;

    let show_reserved_toggle = widget::checkbox(config.show_reserved)
        .label("Show Reserved Space")
        .on_toggle(Message::ToggleShowReserved);

    let volumes_section = widget::container(
        widget::Column::new()
            .push(widget::text::title4("Volumes"))
            .push(show_reserved_toggle)
            .spacing(space_s)
            .align_x(Alignment::Start),
    )
    .width(Length::Fill);

    let parallelism_options = vec![
        fl!("usage-parallelism-low"),
        fl!("usage-parallelism-balanced"),
        fl!("usage-parallelism-high"),
    ];

    let usage_parallelism_dropdown = widget::dropdown(
        parallelism_options,
        Some(config.usage_scan_parallelism.to_index()),
        Message::UsageScanParallelismChanged,
    )
    .width(cosmic::iced::Length::Shrink);

    let usage_section = widget::container(
        widget::Column::new()
            .push(widget::text::title4("Usage"))
            .push(widget::text::caption(fl!("usage-scan-parallelism-label")))
            .push(usage_parallelism_dropdown)
            .spacing(space_s)
            .align_x(Alignment::Start),
    )
    .width(Length::Fill);

    let logging_level_options = vec![
        "Error".to_string(),
        "Warn".to_string(),
        "Info".to_string(),
        "Debug".to_string(),
        "Trace".to_string(),
    ];

    let logging_level_dropdown = widget::dropdown(
        logging_level_options,
        Some(config.log_level.to_index()),
        Message::LogLevelChanged,
    )
    .width(cosmic::iced::Length::Shrink);

    let logging_section = widget::container(
        widget::Column::new()
            .push(widget::text::title4("Logging"))
            .push(widget::text::caption("Log level"))
            .push(logging_level_dropdown)
            .push(
                widget::checkbox(config.log_to_disk)
                    .label("Log to disk")
                    .on_toggle(Message::ToggleLogToDisk),
            )
            .spacing(space_s)
            .align_x(Alignment::Start),
    )
    .width(Length::Fill);

    widget::Column::new()
        .push(volumes_section)
        .push(usage_section)
        .push(logging_section)
        .spacing(space_m)
        .width(Length::Fill)
        .into()
}

pub fn settings_footer<'a>(filesystem_tools: &[FilesystemToolInfo]) -> Element<'a, Message> {
    let cosmic_theme::Spacing {
        space_xxs,
        space_s,
        space_m,
        ..
    } = theme::active().cosmic().spacing;

    let hash = env!("VERGEN_GIT_SHA");
    let short_hash: String = hash.chars().take(7).collect();
    let date = env!("VERGEN_GIT_COMMIT_DATE");

    let commit_hash_link = widget::button::custom(widget::text::caption(short_hash.clone()))
        .class(cosmic::theme::Button::Link)
        .on_press(Message::LaunchUrl(format!("{REPOSITORY}/commits/{hash}")))
        .padding(0);

    let commit_info = widget::row::with_capacity(2)
        .push(widget::text::caption(format!("{} ", date)))
        .push(commit_hash_link)
        .spacing(0)
        .align_y(Alignment::Center);

    let github_icon = widget::icon::icon(
        widget::icon::from_svg_bytes(include_bytes!("../../resources/icons/github.svg"))
            .symbolic(true),
    )
    .size(16);

    let repo_footer = widget::container(
        widget::row::with_capacity(3)
            .push(widget::Space::new().width(Length::Fill))
            .push(commit_info)
            .push(
                widget::button::custom(github_icon)
                    .class(cosmic::theme::Button::Link)
                    .on_press(Message::OpenRepositoryUrl)
                    .padding(0),
            )
            .spacing(space_xxs)
            .align_y(Alignment::Center)
            .width(Length::Fill),
    )
    .padding([0, 0, 3, 0])
    .width(Length::Fill);

    let missing_tools: Vec<_> = filesystem_tools.iter().filter(|t| !t.available).collect();

    if !missing_tools.is_empty() {
        let tools_description = widget::text::caption(fl!("fs-tools-missing-desc"));

        let mut tools_list = widget::Column::new().spacing(space_xxs);
        for tool in &missing_tools {
            let tool_text = widget::text::caption(format!(
                "• {} - {}",
                tool.package_hint,
                fl!("fs-tools-required-for", fs_name = tool.fs_name.clone())
            ));
            tools_list = tools_list.push(tool_text);
        }

        let warning_callout = widget::container(
            widget::Column::new()
                .push(tools_description)
                .push(tools_list)
                .spacing(space_xxs),
        )
        .padding([space_s, space_s, space_s, space_s])
        .width(Length::Fill)
        .style(|theme| {
            let cosmic = theme.cosmic();
            widget::container::Style {
                icon_color: Some(cosmic.warning_color().into()),
                text_color: Some(cosmic.warning_color().into()),
                background: None,
                border: cosmic::iced::Border {
                    color: cosmic.warning_color().into(),
                    width: 1.0,
                    radius: cosmic.corner_radii.radius_s.into(),
                },
                shadow: cosmic::iced::Shadow::default(),
                snap: false,
            }
        });

        widget::Column::new()
            .push(warning_callout)
            .push(repo_footer)
            .spacing(space_s)
            .width(Length::Fill)
            .into()
    } else {
        widget::Column::new()
            .push(repo_footer)
            .spacing(space_m)
            .width(Length::Fill)
            .into()
    }
}
