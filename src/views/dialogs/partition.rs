use crate::app::Message;
use crate::controls::fields::labelled_spinner;
use crate::controls::wizard::{
    WizardBreadcrumbStatus, WizardBreadcrumbStep, wizard_breadcrumb, wizard_step_is_clickable,
    wizard_step_nav, wizard_step_shell,
};
use crate::fl;
use crate::message::dialogs::{
    CreateMessage, EditFilesystemLabelMessage, EditPartitionMessage, ResizePartitionMessage,
};
use crate::state::dialogs::{
    CreatePartitionDialog, CreatePartitionStep, EditFilesystemLabelDialog, EditPartitionDialog,
    EditPartitionStep, FormatPartitionDialog, FormatPartitionStep, ResizePartitionDialog,
    ResizePartitionStep,
};
use crate::utils::SizeUnit;
use cosmic::{
    Element, Theme, iced,
    widget::text::caption,
    widget::{button, checkbox, container, dialog, divider, dropdown, slider, text, text_input},
};
use storage_types::{
    COMMON_DOS_TYPES, COMMON_GPT_TYPES, FilesystemToolInfo, PartitionTypeInfo, bytes_to_pretty,
};

/// Check if a filesystem tool is available from the tools list
fn is_tool_available(tools: &[FilesystemToolInfo], fs_type: &str) -> bool {
    tools
        .iter()
        .find(|t| t.fs_type == fs_type)
        .is_some_and(|t| t.available)
}

pub fn create_partition<'a>(state: CreatePartitionDialog) -> Element<'a, Message> {
    let running = state.running;
    let error = state.error;
    let create = &state.info;
    let current_step = state.step;

    // Get partition type details for radio list
    let partition_types: &[PartitionTypeInfo] = match create.table_type.as_str() {
        "gpt" => &COMMON_GPT_TYPES,
        "dos" => &COMMON_DOS_TYPES,
        _ => &[],
    };

    let mut content = cosmic::iced::widget::column![];
    let mut basics_has_selection = false;

    match current_step {
        CreatePartitionStep::Basics => {
            if create.table_type != "dos" {
                content = content.push(
                    text_input(fl!("volume-name"), create.name.clone())
                        .label(fl!("volume-name"))
                        .on_input(|t| CreateMessage::NameUpdate(t).into()),
                );
                content = content.push(divider::horizontal::default());
            }

            let filesystem_tools = &state.filesystem_tools;
            let available_types: Vec<_> = partition_types
                .iter()
                .filter(|p_type| {
                    let fs_type = p_type.filesystem_type.as_str();
                    is_tool_available(filesystem_tools, fs_type)
                })
                .collect();

            let total_types = partition_types.len();
            let available_count = available_types.len();
            let has_missing_tools = available_count < total_types;

            let dropdown_labels: Vec<String> = available_types
                .iter()
                .map(|p_type| {
                    let fs_type = p_type.filesystem_type.as_str();
                    match fs_type {
                        "ext4" => format!("{} — {}", fl!("fs-name-ext4"), fl!("fs-desc-ext4")),
                        "ext3" => format!("{} — {}", fl!("fs-name-ext3"), fl!("fs-desc-ext3")),
                        "xfs" => format!("{} — {}", fl!("fs-name-xfs"), fl!("fs-desc-xfs")),
                        "btrfs" => format!("{} — {}", fl!("fs-name-btrfs"), fl!("fs-desc-btrfs")),
                        "f2fs" => format!("{} — {}", fl!("fs-name-f2fs"), fl!("fs-desc-f2fs")),
                        "udf" => format!("{} — {}", fl!("fs-name-udf"), fl!("fs-desc-udf")),
                        "ntfs" => format!("{} — {}", fl!("fs-name-ntfs"), fl!("fs-desc-ntfs")),
                        "vfat" => format!("{} — {}", fl!("fs-name-vfat"), fl!("fs-desc-vfat")),
                        "exfat" => format!("{} — {}", fl!("fs-name-exfat"), fl!("fs-desc-exfat")),
                        "swap" => format!("{} — {}", fl!("fs-name-swap"), fl!("fs-desc-swap")),
                        fs => fs.to_string(),
                    }
                })
                .collect();

            let selected_in_filtered = available_types.iter().position(|p| {
                let original_idx = partition_types
                    .iter()
                    .position(|orig| orig.filesystem_type == p.filesystem_type);
                original_idx == Some(create.selected_partition_type_index)
            });

            basics_has_selection = selected_in_filtered.is_some();

            content = content.push(caption(fl!("filesystem-type")));
            content = content.push(dropdown(
                dropdown_labels,
                selected_in_filtered,
                move |selected_idx| {
                    let original_idx = partition_types
                        .iter()
                        .position(|orig| {
                            orig.filesystem_type == available_types[selected_idx].filesystem_type
                        })
                        .unwrap_or(0);
                    CreateMessage::PartitionTypeUpdate(original_idx).into()
                },
            ));

            if has_missing_tools {
                let warning_text = container(caption(format!("⚠ {}", fl!("fs-tools-warning"))))
                    .style(|theme: &Theme| container::Style {
                        text_color: Some(theme.cosmic().warning_color().into()),
                        ..Default::default()
                    });
                content = content.push(warning_text);
            }
        }
        CreatePartitionStep::Sizing => {
            let len = create.max_size as f64;
            let size = create.size as f64;
            let free = len - size;
            let free_bytes = free as u64;

            let selected_unit = SizeUnit::from_index(create.size_unit_index);
            let size_in_unit = selected_unit.from_bytes(create.size);
            let free_in_unit = selected_unit.from_bytes(free_bytes);
            let max_in_unit = selected_unit.from_bytes(create.max_size);

            let size_text = if size_in_unit < 10.0 {
                format!("{:.2}", size_in_unit)
            } else {
                format!("{:.1}", size_in_unit)
            };

            let free_text = if free_in_unit < 10.0 {
                format!("{:.2}", free_in_unit)
            } else {
                format!("{:.1}", free_in_unit)
            };

            let step = storage_types::get_step(&create.size);
            let current_size = create.size;
            let max_size = create.max_size;

            content = content.push(slider(0.0..=len, size, |v| {
                CreateMessage::SizeUpdate(v as u64).into()
            }));

            let label_width = iced::Length::Fixed(120.);

            let size_row = cosmic::iced::widget::row![
                text(fl!("partition-size")).width(label_width),
                button::text("-")
                    .on_press(CreateMessage::SizeUpdate((size - step).max(0.) as u64).into()),
                text_input("", size_text)
                    .width(iced::Length::Fixed(100.))
                    .on_input(move |v| match v.trim().parse::<f64>() {
                        Ok(value) if value >= 0.0 => {
                            let bytes = selected_unit.to_bytes(value.min(max_in_unit));
                            CreateMessage::SizeUpdate(bytes.min(max_size)).into()
                        }
                        _ => CreateMessage::SizeUpdate(current_size).into(),
                    }),
                button::text("+")
                    .on_press(CreateMessage::SizeUpdate((size + step).min(len) as u64).into()),
                dropdown(SizeUnit::labels(), Some(create.size_unit_index), |idx| {
                    CreateMessage::SizeUnitUpdate(idx).into()
                })
                .width(iced::Length::Fixed(80.)),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center);

            let free_row = cosmic::iced::widget::row![
                text(fl!("free-space")).width(label_width),
                button::text("-")
                    .on_press(CreateMessage::SizeUpdate((size + step).min(len) as u64).into()),
                text_input("", free_text)
                    .width(iced::Length::Fixed(100.))
                    .on_input(move |v| match v.trim().parse::<f64>() {
                        Ok(free_value) if free_value >= 0.0 => {
                            let free_bytes = selected_unit.to_bytes(free_value.min(max_in_unit));
                            let new_size = max_size.saturating_sub(free_bytes);
                            CreateMessage::SizeUpdate(new_size).into()
                        }
                        _ => CreateMessage::SizeUpdate(current_size).into(),
                    }),
                button::text("+")
                    .on_press(CreateMessage::SizeUpdate((size - step).max(0.) as u64).into()),
                dropdown(SizeUnit::labels(), Some(create.size_unit_index), |idx| {
                    CreateMessage::SizeUnitUpdate(idx).into()
                })
                .width(iced::Length::Fixed(80.)),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center);

            content = content.push(size_row);
            content = content.push(free_row);
        }
        CreatePartitionStep::Options => {
            content = content.push(
                checkbox(create.erase)
                    .label(fl!("overwrite-data-slow"))
                    .on_toggle(|v| CreateMessage::EraseUpdate(v).into()),
            );

            content = content.push(
                checkbox(create.password_protected)
                    .label(fl!("password-protected-luks"))
                    .on_toggle(|v| CreateMessage::PasswordProtectedUpdate(v).into()),
            );

            if create.password_protected {
                content = content.push(
                    text_input::secure_input("", create.password.clone(), None, true)
                        .label(fl!("password"))
                        .on_input(|v| CreateMessage::PasswordUpdate(v).into()),
                );

                content = content.push(
                    text_input::secure_input("", create.confirmed_password.clone(), None, true)
                        .label(fl!("confirm"))
                        .on_input(|v| CreateMessage::ConfirmedPasswordUpdate(v).into()),
                );
            }
        }
    }

    if let Some(err) = error.as_ref() {
        content = content.push(caption(err.clone()));
    }

    if running {
        content = content.push(caption(fl!("working")));
    }

    let sizing_valid = create.size > 0 && create.size <= create.max_size;
    let options_valid = !create.password_protected
        || (!create.password.is_empty() && create.password == create.confirmed_password);

    let can_advance = match current_step {
        CreatePartitionStep::Basics => basics_has_selection,
        CreatePartitionStep::Sizing => sizing_valid,
        CreatePartitionStep::Options => false,
    };

    let can_apply = current_step == CreatePartitionStep::Options && options_valid && !running;

    let current_number = current_step.number();
    let steps = [
        (CreatePartitionStep::Basics, "Details".to_string()),
        (CreatePartitionStep::Sizing, "Sizing".to_string()),
        (CreatePartitionStep::Options, "Options".to_string()),
    ];

    let breadcrumb = wizard_breadcrumb(
        steps
            .iter()
            .map(|(step, label)| {
                let step_number = step.number();
                let status = if step_number == current_number {
                    WizardBreadcrumbStatus::Current
                } else if step_number < current_number {
                    WizardBreadcrumbStatus::Completed
                } else {
                    WizardBreadcrumbStatus::Upcoming
                };

                let on_press = if wizard_step_is_clickable(step_number, current_number) {
                    Some(CreateMessage::SetStep(*step).into())
                } else {
                    None
                };

                WizardBreadcrumbStep {
                    label: label.clone(),
                    status,
                    on_press,
                }
            })
            .collect(),
    );

    let back_message = if current_step == CreatePartitionStep::Basics {
        None
    } else {
        Some(CreateMessage::PrevStep.into())
    };

    let (primary_label, primary_message) = if current_step == CreatePartitionStep::Options {
        (
            fl!("apply"),
            if can_apply {
                Some(CreateMessage::Partition.into())
            } else {
                None
            },
        )
    } else {
        (
            "Next".to_string(),
            if can_advance && !running {
                Some(CreateMessage::NextStep.into())
            } else {
                None
            },
        )
    };

    let footer = wizard_step_nav(
        CreateMessage::Cancel.into(),
        back_message,
        primary_label,
        primary_message,
    );

    wizard_step_shell(
        caption(fl!("create-partition")).into(),
        breadcrumb,
        content.spacing(20.).into(),
        footer,
    )
}

pub fn format_partition<'a>(state: FormatPartitionDialog) -> Element<'a, Message> {
    let FormatPartitionDialog {
        volume: _,
        info: create,
        step,
        running,
        filesystem_tools,
    } = state;

    let size_pretty = bytes_to_pretty(&create.size, false);

    // Get partition type details for radio list
    let partition_types: &[PartitionTypeInfo] = match create.table_type.as_str() {
        "gpt" => &COMMON_GPT_TYPES,
        "dos" => &COMMON_DOS_TYPES,
        _ => &[],
    };

    let mut content = cosmic::iced::widget::column![caption(fl!(
        "format-partition-description",
        size = size_pretty
    )),];

    // Get filesystem tool availability from service data
    // Filter partition types to only include those with available tools
    let available_types: Vec<_> = partition_types
        .iter()
        .filter(|p_type| {
            let fs_type = p_type.filesystem_type.as_str();
            is_tool_available(&filesystem_tools, fs_type)
        })
        .collect();

    let total_types = partition_types.len();
    let available_count = available_types.len();
    let has_missing_tools = available_count < total_types;

    // Build dropdown labels for available types
    let dropdown_labels: Vec<String> = available_types
        .iter()
        .map(|p_type| {
            let fs_type = p_type.filesystem_type.as_str();
            match fs_type {
                "ext4" => format!("{} — {}", fl!("fs-name-ext4"), fl!("fs-desc-ext4")),
                "ext3" => format!("{} — {}", fl!("fs-name-ext3"), fl!("fs-desc-ext3")),
                "xfs" => format!("{} — {}", fl!("fs-name-xfs"), fl!("fs-desc-xfs")),
                "btrfs" => format!("{} — {}", fl!("fs-name-btrfs"), fl!("fs-desc-btrfs")),
                "f2fs" => format!("{} — {}", fl!("fs-name-f2fs"), fl!("fs-desc-f2fs")),
                "udf" => format!("{} — {}", fl!("fs-name-udf"), fl!("fs-desc-udf")),
                "ntfs" => format!("{} — {}", fl!("fs-name-ntfs"), fl!("fs-desc-ntfs")),
                "vfat" => format!("{} — {}", fl!("fs-name-vfat"), fl!("fs-desc-vfat")),
                "exfat" => format!("{} — {}", fl!("fs-name-exfat"), fl!("fs-desc-exfat")),
                "swap" => format!("{} — {}", fl!("fs-name-swap"), fl!("fs-desc-swap")),
                fs => fs.to_string(),
            }
        })
        .collect();

    // Map selected index from full list to filtered list
    let selected_in_filtered = available_types.iter().position(|p| {
        let original_idx = partition_types
            .iter()
            .position(|orig| orig.filesystem_type == p.filesystem_type);
        original_idx == Some(create.selected_partition_type_index)
    });

    if step == FormatPartitionStep::Basics {
        if create.table_type != "dos" {
            content = content.push(
                text_input(fl!("volume-name"), create.name.clone())
                    .label(fl!("volume-name"))
                    .on_input(|t| CreateMessage::NameUpdate(t).into()),
            );
        }

        content = content.push(caption(fl!("filesystem-type")));
        content = content.push(dropdown(
            dropdown_labels,
            selected_in_filtered,
            move |selected_idx| {
                let original_idx = partition_types
                    .iter()
                    .position(|orig| {
                        orig.filesystem_type == available_types[selected_idx].filesystem_type
                    })
                    .unwrap_or(0);
                CreateMessage::PartitionTypeUpdate(original_idx).into()
            },
        ));

        if has_missing_tools {
            let warning_text = container(caption(format!("⚠ {}", fl!("fs-tools-warning")))).style(
                |theme: &Theme| container::Style {
                    text_color: Some(theme.cosmic().warning_color().into()),
                    ..Default::default()
                },
            );
            content = content.push(warning_text);
        }
    } else {
        content = content.push(
            checkbox(create.erase)
                .label(fl!("overwrite-data-slow"))
                .on_toggle(|v| CreateMessage::EraseUpdate(v).into()),
        );
    }

    content = content.spacing(12);

    if running {
        content = content.push(caption(fl!("working")));
    }

    let current_number = step.number();
    let steps = [
        (FormatPartitionStep::Basics, "Details".to_string()),
        (FormatPartitionStep::Options, "Options".to_string()),
    ];

    let breadcrumb = wizard_breadcrumb(
        steps
            .iter()
            .map(|(wizard_step, label)| {
                let number = wizard_step.number();
                let status = if number == current_number {
                    WizardBreadcrumbStatus::Current
                } else if number < current_number {
                    WizardBreadcrumbStatus::Completed
                } else {
                    WizardBreadcrumbStatus::Upcoming
                };
                let on_press = if wizard_step_is_clickable(number, current_number) {
                    Some(CreateMessage::SetFormatStep(*wizard_step).into())
                } else {
                    None
                };

                WizardBreadcrumbStep {
                    label: label.clone(),
                    status,
                    on_press,
                }
            })
            .collect(),
    );

    let back_message = if step == FormatPartitionStep::Basics {
        None
    } else {
        Some(CreateMessage::PrevStep.into())
    };

    let (primary_label, primary_message) = if step == FormatPartitionStep::Basics {
        (
            "Next".to_string(),
            if selected_in_filtered.is_some() && !running {
                Some(CreateMessage::NextStep.into())
            } else {
                None
            },
        )
    } else {
        (
            fl!("apply"),
            if !running {
                Some(CreateMessage::Partition.into())
            } else {
                None
            },
        )
    };

    let footer = wizard_step_nav(
        CreateMessage::Cancel.into(),
        back_message,
        primary_label,
        primary_message,
    );

    wizard_step_shell(
        caption(fl!("format-partition")).into(),
        breadcrumb,
        content.into(),
        footer,
    )
}

pub fn edit_partition<'a>(state: EditPartitionDialog) -> Element<'a, Message> {
    let EditPartitionDialog {
        volume: _,
        step,
        partition_types,
        selected_type_index,
        name,
        legacy_bios_bootable,
        system_partition,
        hidden,
        running,
    } = state;

    let opts: Vec<String> = partition_types
        .iter()
        .map(|t: &PartitionTypeInfo| format!("{} - {}", t.name, t.ty))
        .collect();

    let mut content = cosmic::iced::widget::column![].spacing(12);

    match step {
        EditPartitionStep::Basics => {
            content = content.push(dropdown(opts, Some(selected_type_index), |v| {
                EditPartitionMessage::TypeUpdate(v).into()
            }));
            content = content.push(
                text_input(fl!("partition-name"), name)
                    .label(fl!("partition-name"))
                    .on_input(|t| EditPartitionMessage::NameUpdate(t).into()),
            );
        }
        EditPartitionStep::Flags => {
            content = content.push(
                checkbox(legacy_bios_bootable)
                    .label(fl!("flag-legacy-bios-bootable"))
                    .on_toggle(|v| EditPartitionMessage::LegacyBiosBootableUpdate(v).into()),
            );
            content = content.push(
                checkbox(system_partition)
                    .label(fl!("flag-system-partition"))
                    .on_toggle(|v| EditPartitionMessage::SystemPartitionUpdate(v).into()),
            );
            content = content.push(
                checkbox(hidden)
                    .label(fl!("flag-hide-from-firmware"))
                    .on_toggle(|v| EditPartitionMessage::HiddenUpdate(v).into()),
            );
        }
        EditPartitionStep::Review => {
            let selected_type = partition_types
                .get(selected_type_index)
                .map(|t| format!("{} - {}", t.name, t.ty))
                .unwrap_or_else(|| fl!("unknown"));

            content = content
                .push(caption(format!("{}: {}", fl!("partition-name"), name)))
                .push(caption(format!(
                    "{}: {}",
                    fl!("partition-type"),
                    selected_type
                )))
                .push(caption(format!(
                    "{}: {}",
                    fl!("flag-legacy-bios-bootable"),
                    if legacy_bios_bootable { "Yes" } else { "No" }
                )))
                .push(caption(format!(
                    "{}: {}",
                    fl!("flag-system-partition"),
                    if system_partition { "Yes" } else { "No" }
                )))
                .push(caption(format!(
                    "{}: {}",
                    fl!("flag-hide-from-firmware"),
                    if hidden { "Yes" } else { "No" }
                )));
        }
    }

    if running {
        content = content.push(caption(fl!("working")));
    }

    let current_number = step.number();
    let steps = [
        (EditPartitionStep::Basics, "Details".to_string()),
        (EditPartitionStep::Flags, "Flags".to_string()),
        (EditPartitionStep::Review, "Review".to_string()),
    ];

    let breadcrumb = wizard_breadcrumb(
        steps
            .iter()
            .map(|(wizard_step, label)| {
                let number = wizard_step.number();
                let status = if number == current_number {
                    WizardBreadcrumbStatus::Current
                } else if number < current_number {
                    WizardBreadcrumbStatus::Completed
                } else {
                    WizardBreadcrumbStatus::Upcoming
                };
                let on_press = if wizard_step_is_clickable(number, current_number) {
                    Some(EditPartitionMessage::SetStep(*wizard_step).into())
                } else {
                    None
                };

                WizardBreadcrumbStep {
                    label: label.clone(),
                    status,
                    on_press,
                }
            })
            .collect(),
    );

    let back_message = if step == EditPartitionStep::Basics {
        None
    } else {
        Some(EditPartitionMessage::PrevStep.into())
    };

    let (primary_label, primary_message) = if step == EditPartitionStep::Review {
        (
            fl!("apply"),
            if !running {
                Some(EditPartitionMessage::Confirm.into())
            } else {
                None
            },
        )
    } else {
        (
            "Next".to_string(),
            if !running {
                Some(EditPartitionMessage::NextStep.into())
            } else {
                None
            },
        )
    };

    let footer = wizard_step_nav(
        EditPartitionMessage::Cancel.into(),
        back_message,
        primary_label,
        primary_message,
    );

    wizard_step_shell(
        caption(fl!("edit-partition")).into(),
        breadcrumb,
        content.into(),
        footer,
    )
}

pub fn resize_partition<'a>(state: ResizePartitionDialog) -> Element<'a, Message> {
    let ResizePartitionDialog {
        volume: _,
        step: wizard_step,
        min_size_bytes,
        max_size_bytes,
        new_size_bytes,
        running,
    } = state;

    let min = min_size_bytes as f64;
    let max = max_size_bytes as f64;
    let value = new_size_bytes as f64;
    let step = storage_types::get_step(&new_size_bytes);

    let min_pretty = bytes_to_pretty(&min_size_bytes, false);
    let max_pretty = bytes_to_pretty(&max_size_bytes, false);
    let value_pretty = bytes_to_pretty(&new_size_bytes, false);

    let can_resize = max_size_bytes.saturating_sub(min_size_bytes) >= 1024;

    let mut content = cosmic::iced::widget::column![].spacing(12);

    if wizard_step == ResizePartitionStep::Sizing {
        content = content
            .push(caption(fl!(
                "resize-partition-range",
                min = min_pretty,
                max = max_pretty
            )))
            .push(slider(min..=max, value, |v| {
                ResizePartitionMessage::SizeUpdate(v as u64).into()
            }))
            .push(labelled_spinner(
                fl!("new-size"),
                value_pretty,
                value,
                step,
                min,
                max,
                |v| ResizePartitionMessage::SizeUpdate(v as u64).into(),
            ));
    } else {
        content = content
            .push(caption(fl!(
                "resize-partition-range",
                min = min_pretty,
                max = max_pretty
            )))
            .push(caption(format!("{}: {}", fl!("new-size"), value_pretty)));
    }

    if running {
        content = content.push(caption(fl!("working")));
    }

    let current_number = wizard_step.number();
    let steps = [
        (ResizePartitionStep::Sizing, "Sizing".to_string()),
        (ResizePartitionStep::Review, "Review".to_string()),
    ];

    let breadcrumb = wizard_breadcrumb(
        steps
            .iter()
            .map(|(step_state, label)| {
                let number = step_state.number();
                let status = if number == current_number {
                    WizardBreadcrumbStatus::Current
                } else if number < current_number {
                    WizardBreadcrumbStatus::Completed
                } else {
                    WizardBreadcrumbStatus::Upcoming
                };
                let on_press = if wizard_step_is_clickable(number, current_number) {
                    Some(ResizePartitionMessage::SetStep(*step_state).into())
                } else {
                    None
                };

                WizardBreadcrumbStep {
                    label: label.clone(),
                    status,
                    on_press,
                }
            })
            .collect(),
    );

    let back_message = if wizard_step == ResizePartitionStep::Sizing {
        None
    } else {
        Some(ResizePartitionMessage::PrevStep.into())
    };

    let (primary_label, primary_message) = if wizard_step == ResizePartitionStep::Review {
        (
            fl!("apply"),
            if !running && can_resize {
                Some(ResizePartitionMessage::Confirm.into())
            } else {
                None
            },
        )
    } else {
        (
            "Next".to_string(),
            if !running && can_resize {
                Some(ResizePartitionMessage::NextStep.into())
            } else {
                None
            },
        )
    };

    let footer = wizard_step_nav(
        ResizePartitionMessage::Cancel.into(),
        back_message,
        primary_label,
        primary_message,
    );

    wizard_step_shell(
        caption(fl!("resize-partition")).into(),
        breadcrumb,
        content.into(),
        footer,
    )
}

pub fn edit_filesystem_label<'a>(state: EditFilesystemLabelDialog) -> Element<'a, Message> {
    let EditFilesystemLabelDialog {
        target: _,
        label,
        running,
    } = state;

    let mut content = cosmic::iced::widget::column![
        text_input(fl!("filesystem-label"), label)
            .label(fl!("filesystem-label"))
            .on_input(|t| EditFilesystemLabelMessage::LabelUpdate(t).into()),
    ]
    .spacing(12);

    if running {
        content = content.push(caption(fl!("working")));
    }

    let mut apply = button::standard(fl!("apply"));
    if !running {
        apply = apply.on_press(EditFilesystemLabelMessage::Confirm.into());
    }

    dialog::dialog()
        .title(fl!("edit-filesystem"))
        .control(content)
        .primary_action(apply)
        .secondary_action(
            button::standard(fl!("cancel")).on_press(EditFilesystemLabelMessage::Cancel.into()),
        )
        .into()
}
