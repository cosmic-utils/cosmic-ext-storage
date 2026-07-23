// SPDX-License-Identifier: GPL-3.0-only

//! View components for network mount management

use crate::controls::actions::{icon_tooltip_action, trailing_actions_row};
use crate::controls::form::bounded_form;
use crate::controls::icons::{resolve_provider_icon, scope_icon, scope_label};
use crate::controls::layout::{row_container, transparent_button_class};
use crate::controls::wizard::{
    WizardBreadcrumbStatus, WizardBreadcrumbStep, option_tile_grid, selectable_tile,
    wizard_breadcrumb, wizard_shell, wizard_step_nav,
};
use crate::message::network::NetworkMessage;
use crate::state::network::{
    NetworkEditorState, NetworkMountState, NetworkState, NetworkWizardState, QUICK_SETUP_PROVIDERS,
    SECTION_ORDER, WizardStep, section_display_name,
};
use cosmic::cosmic_theme::palette::WithAlpha;
use cosmic::iced::Length;
use cosmic::iced::widget as iced_widget;
use cosmic::widget::{self, button, dropdown, icon, text_input};
use cosmic::{Apply, Element};
use std::collections::BTreeMap;
use storage_types::rclone::{ConfigScope, MountStatus, rclone_provider, supported_remote_types};

// ─── Sidebar helpers ─────────────────────────────────────────────────────────

fn provider_logo_widget(provider_type: &str, size: u16) -> Element<'static, NetworkMessage> {
    let provider_icon = resolve_provider_icon(provider_type);
    if let Some(brand_icon) = provider_icon.branded {
        return icon::icon(icon::from_svg_bytes(brand_icon.svg_bytes()).symbolic(true))
            .size(size)
            .into();
    }

    if let Some(text_fallback) = provider_icon.text_fallback {
        return widget::container(
            widget::text::caption(text_fallback.to_string())
                .font(cosmic::font::semibold())
                .align_x(cosmic::iced::alignment::Horizontal::Center),
        )
        .width(size)
        .height(size)
        .align_x(cosmic::iced::alignment::Horizontal::Center)
        .align_y(cosmic::iced::alignment::Vertical::Center)
        .into();
    }

    icon::from_name(provider_icon.fallback_symbolic)
        .size(size)
        .into()
}

/// Render a single network mount item for the sidebar
pub fn network_mount_item(
    state: &NetworkState,
    name: &str,
    scope: ConfigScope,
    controls_enabled: bool,
) -> Element<'static, NetworkMessage> {
    let mount = match state.get_mount(name, scope) {
        Some(m) => m,
        None => return widget::text::body("Unknown mount").into(),
    };

    // Extract all data needed before building UI
    let selected = state.is_selected(name, scope);
    let loading = mount.loading;
    let config_name = mount.config.name.clone();

    // Name text
    let name_text = widget::text::body(config_name).font(cosmic::font::semibold());

    // Scope icon (left aligned)
    let provider_icon_widget = provider_logo_widget(&mount.config.remote_type, 16);

    let scope_icon_widget = widget::tooltip(
        icon::from_name(scope_icon(scope)).size(14),
        widget::text::body(scope_label(scope)),
        widget::tooltip::Position::FollowCursor,
    );

    // Main select button
    let mut select_button = widget::button::custom(
        widget::Row::with_children(vec![
            provider_icon_widget,
            name_text.into(),
            scope_icon_widget.into(),
        ])
        .spacing(8)
        .align_y(cosmic::iced::Alignment::Center)
        .width(Length::Fill),
    )
    .padding(0)
    .width(Length::Fill)
    .class(transparent_button_class(selected));

    if controls_enabled && !loading {
        select_button = select_button.on_press(NetworkMessage::SelectRemote {
            name: name.to_string(),
            scope,
        });
    }

    // Compose the row
    let row = widget::Row::with_children(vec![
        widget::Space::new().width(20).into(), // Indent to match drive tree
        select_button.into(),
    ])
    .spacing(8)
    .align_y(cosmic::iced::Alignment::Center)
    .width(Length::Fill);

    row_container(row, selected, controls_enabled)
}

/// Section header for sidebar
fn sidebar_section_header(label: &str, controls_enabled: bool) -> Element<'static, NetworkMessage> {
    let label_widget = widget::text::caption_heading(label.to_string());

    let mut children: Vec<Element<'static, NetworkMessage>> = vec![label_widget.into()];

    if controls_enabled {
        let add_btn = widget::button::custom(icon::from_name("list-add-symbolic").size(20))
            .padding(4)
            .class(cosmic::theme::Button::Link)
            .on_press(NetworkMessage::BeginCreateRemote);
        children.push(widget::Space::new().width(Length::Fill).into());
        children.push(add_btn.into());
    }

    widget::Row::with_children(children)
        .padding([8, 12, 4, 12])
        .align_y(cosmic::iced::Alignment::Center)
        .into()
}

/// Render the Network section for the sidebar
pub fn network_section(
    state: &NetworkState,
    controls_enabled: bool,
) -> Element<'static, NetworkMessage> {
    let mut children: Vec<Element<'static, NetworkMessage>> = Vec::new();

    // Header
    children.push(sidebar_section_header("Network", controls_enabled));

    // Loading state
    if state.loading {
        children.push(
            widget::container(widget::text::body("Loading remotes..."))
                .padding([4, 12])
                .into(),
        );
    } else if !state.rclone_available {
        children.push(
            widget::container(widget::text::body("RClone not available"))
                .padding([4, 12])
                .into(),
        );
    } else if state.mounts.is_empty() {
        // Empty state
        children.push(
            widget::container(widget::text::body("No network mounts configured"))
                .padding([4, 12])
                .into(),
        );
    } else {
        // Collect mount info to avoid borrowing issues
        let mount_info: Vec<(String, ConfigScope)> = state
            .mounts
            .values()
            .map(|m| (m.config.name.clone(), m.config.scope))
            .collect();

        // Separate user and system mounts
        let (user_mounts, system_mounts): (Vec<_>, Vec<_>) = mount_info
            .into_iter()
            .partition(|(_, scope)| *scope == ConfigScope::User);

        let has_user_mounts = !user_mounts.is_empty();

        for (name, scope) in &user_mounts {
            children.push(network_mount_item(state, name, *scope, controls_enabled));
        }

        if !system_mounts.is_empty() {
            if has_user_mounts {
                children.push(widget::Space::new().height(4).into());
            }
            for (name, scope) in &system_mounts {
                children.push(network_mount_item(state, name, *scope, controls_enabled));
            }
        }
    }

    widget::column::with_children(children)
        .spacing(2)
        .width(Length::Fill)
        .into()
}

// ─── Shared helpers ──────────────────────────────────────────────────────────

/// Convert a snake_case field name to Title Case for display.
/// e.g. "access_key_id" -> "Access Key Id", "host" -> "Host"
fn pretty_field_name(name: &str) -> String {
    name.split('_')
        .filter(|s| !s.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => {
                    let upper = first.to_uppercase().to_string();
                    format!("{}{}", upper, chars.as_str())
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn field_placeholder(option: &storage_types::rclone::RcloneProviderOption) -> String {
    if let Some(example) = option.examples.first() {
        return example.value.clone();
    }
    if !option.default_value.is_empty() {
        return option.default_value.clone();
    }
    String::new()
}

fn status_label(status: &MountStatus) -> &'static str {
    match status {
        MountStatus::Mounted => "Mounted",
        MountStatus::Unmounted => "Unmounted",
        MountStatus::Mounting => "Mounting",
        MountStatus::Unmounting => "Unmounting",
        MountStatus::Error(_) => "Error",
    }
}

fn status_badge(text: String, running: bool, unsaved: bool) -> Element<'static, NetworkMessage> {
    use cosmic::iced::widget::container;

    let badge = widget::text::caption(text);

    widget::container(badge)
        .style(move |theme| {
            let mut color = theme
                .cosmic()
                .background(false)
                .component
                .on
                .with_alpha(0.6);
            if running {
                color = theme.cosmic().accent_color();
            } else if unsaved {
                color = theme.cosmic().warning_color();
            }

            container::Style {
                text_color: Some(color.into()),
                ..Default::default()
            }
        })
        .into()
}

/// Build a single option field widget (used by both editor and wizard)
fn option_field_widget<F>(
    option: &storage_types::rclone::RcloneProviderOption,
    value: &str,
    on_change: F,
) -> Element<'static, NetworkMessage>
where
    F: Fn(String) -> NetworkMessage + 'static,
{
    let display_name = pretty_field_name(&option.name);
    let label = if option.required {
        format!("{} *", display_name)
    } else {
        display_name
    };

    let placeholder = field_placeholder(option);
    let input = if option.is_secure() {
        text_input::secure_input(placeholder, value.to_owned(), None, true)
            .label(label)
            .on_input(on_change)
    } else {
        text_input(placeholder, value.to_owned())
            .label(label)
            .on_input(on_change)
    };

    let mut col = iced_widget::column![input].spacing(4);

    let help = option.help.trim();
    if !help.is_empty() {
        col = col.push(widget::text::caption(help.to_string()));
    }

    col.into()
}

/// Expander icon helper
fn expander_icon(expanded: bool) -> &'static str {
    if expanded {
        "go-down-symbolic"
    } else {
        "go-next-symbolic"
    }
}

// ─── Editor (for existing remotes and advanced new-remote) ───────────────────

fn editor_header(
    editor: &NetworkEditorState,
    selected_mount: Option<&NetworkMountState>,
    controls_enabled: bool,
) -> Element<'static, NetworkMessage> {
    let title = if editor.name.trim().is_empty() {
        "New Remote".to_string()
    } else {
        editor.name.clone()
    };

    let status_text = selected_mount
        .map(|m| status_label(&m.status).to_string())
        .unwrap_or_else(|| "Not saved".to_string());
    let unsaved = editor.is_new || selected_mount.is_none();
    let status_badge = status_badge(status_text, editor.running, unsaved);

    let can_control =
        selected_mount.is_some_and(|m| !m.loading) && controls_enabled && !editor.running;
    let is_mounted = selected_mount.map(|m| m.is_mounted()).unwrap_or(false);

    let mut actions: Vec<Element<'static, NetworkMessage>> = Vec::new();

    if let Some(mount) = selected_mount {
        actions.push(icon_tooltip_action(
            "media-playback-start-symbolic",
            "Start",
            Some(NetworkMessage::MountRemote {
                name: mount.config.name.clone(),
                scope: mount.config.scope,
            }),
            can_control && !is_mounted,
        ));

        actions.push(icon_tooltip_action(
            "media-playback-stop-symbolic",
            "Stop",
            Some(NetworkMessage::UnmountRemote {
                name: mount.config.name.clone(),
                scope: mount.config.scope,
            }),
            can_control && is_mounted,
        ));

        actions.push(icon_tooltip_action(
            "view-refresh-symbolic",
            "Restart",
            Some(NetworkMessage::RestartRemote {
                name: mount.config.name.clone(),
                scope: mount.config.scope,
            }),
            can_control,
        ));

        actions.push(icon_tooltip_action(
            "emblem-system-symbolic",
            "Test",
            Some(NetworkMessage::TestRemote {
                name: mount.config.name.clone(),
                scope: mount.config.scope,
            }),
            can_control,
        ));
    } else {
        actions.push(icon_tooltip_action(
            "media-playback-start-symbolic",
            "Start",
            None,
            false,
        ));
        actions.push(icon_tooltip_action(
            "media-playback-stop-symbolic",
            "Stop",
            None,
            false,
        ));
        actions.push(icon_tooltip_action(
            "view-refresh-symbolic",
            "Restart",
            None,
            false,
        ));
        actions.push(icon_tooltip_action(
            "emblem-system-symbolic",
            "Test",
            None,
            false,
        ));
    }

    actions.push(icon_tooltip_action(
        "document-save-symbolic",
        if editor.is_new { "Create" } else { "Save" },
        Some(NetworkMessage::SaveRemote),
        controls_enabled && !editor.running,
    ));

    if !editor.is_new {
        let delete_message = selected_mount.map(|mount| NetworkMessage::DeleteRemote {
            name: mount.config.name.clone(),
            scope: mount.config.scope,
        });
        actions.push(icon_tooltip_action(
            "edit-delete-symbolic",
            "Delete",
            delete_message,
            controls_enabled && !editor.running && selected_mount.is_some(),
        ));
    }

    let actions_row = trailing_actions_row(actions);

    iced_widget::row![
        widget::text::title2(title),
        widget::Space::new().width(Length::Fill),
        status_badge,
        actions_row
    ]
    .spacing(12)
    .align_y(cosmic::iced::Alignment::Center)
    .into()
}

/// Build a collapsible section expander for the editor
fn section_expander(
    section_id: &str,
    display_name: &str,
    field_count: usize,
    expanded: bool,
) -> Element<'static, NetworkMessage> {
    let section_id = section_id.to_string();

    let expander_row = iced_widget::row![
        icon::from_name(expander_icon(expanded)).size(16),
        widget::text::body(format!("{display_name} ({field_count})"))
            .font(cosmic::font::semibold()),
    ]
    .spacing(8)
    .align_y(cosmic::iced::Alignment::Center);

    widget::button::custom(expander_row)
        .padding([8, 4])
        .width(Length::Fill)
        .class(cosmic::theme::Button::Text)
        .on_press(NetworkMessage::EditorToggleSection(section_id))
        .into()
}

fn editor_form(
    editor: &NetworkEditorState,
    provider: Option<&storage_types::rclone::RcloneProvider>,
    _controls_enabled: bool,
) -> Element<'static, NetworkMessage> {
    let remote_types: Vec<String> = supported_remote_types().to_vec();
    let remote_type_index = remote_types
        .iter()
        .position(|t| t.eq_ignore_ascii_case(&editor.remote_type))
        .unwrap_or(0);

    let mut form = iced_widget::column![
        text_input("Enter remote name", editor.name.clone())
            .label("Remote Name")
            .on_input(NetworkMessage::EditorNameChanged),
        widget::text::caption("Remote Type"),
        dropdown(remote_types, Some(remote_type_index), |idx| {
            NetworkMessage::EditorTypeIndexChanged(idx)
        })
        .width(Length::Fill),
        widget::text::caption("Network-drive configurations are stored for this desktop user.")
    ]
    .spacing(10);

    // Show advanced / hidden toggle row
    if let Some(provider) = provider {
        let has_advanced = provider
            .options
            .iter()
            .any(|o| o.advanced && !o.is_hidden());
        let has_hidden = provider.options.iter().any(|o| o.is_hidden());

        if has_advanced || has_hidden {
            let mut toggle_row: Vec<Element<'static, NetworkMessage>> = Vec::new();

            if has_advanced {
                toggle_row.push(
                    widget::checkbox(editor.show_advanced)
                        .label("Show advanced options")
                        .on_toggle(NetworkMessage::EditorShowAdvanced)
                        .into(),
                );
            }
            if has_hidden {
                toggle_row.push(
                    widget::checkbox(editor.show_hidden)
                        .label("Show internal options")
                        .on_toggle(NetworkMessage::EditorShowHidden)
                        .into(),
                );
            }

            form = form.push(
                widget::Row::from_vec(toggle_row)
                    .spacing(20)
                    .align_y(cosmic::iced::Alignment::Center),
            );
        }
    }

    // Group options by section
    if let Some(provider) = provider {
        // Collect options into sections, respecting visibility filters
        let mut sections: BTreeMap<
            usize,
            (&str, Vec<&storage_types::rclone::RcloneProviderOption>),
        > = BTreeMap::new();

        for option in &provider.options {
            // Skip hidden options unless show_hidden is on
            if option.is_hidden() && !editor.show_hidden {
                continue;
            }
            // Skip advanced options unless show_advanced is on
            if option.advanced && !option.is_hidden() && !editor.show_advanced {
                continue;
            }

            let section = option.section.as_str();
            let order = SECTION_ORDER
                .iter()
                .position(|s| *s == section)
                .unwrap_or(SECTION_ORDER.len());

            sections
                .entry(order)
                .or_insert_with(|| (section, Vec::new()))
                .1
                .push(option);
        }

        for (section_id, options) in sections.values() {
            let display_name = section_display_name(section_id);
            let expanded = editor.expanded_sections.contains(*section_id);
            let field_count = options.len();

            form = form.push(section_expander(
                section_id,
                display_name,
                field_count,
                expanded,
            ));

            if expanded {
                let fields: Vec<Element<'static, NetworkMessage>> = options
                    .iter()
                    .map(|option| {
                        let option_name = option.name.clone();
                        let value = editor
                            .options
                            .get(&option_name)
                            .cloned()
                            .unwrap_or_default();
                        option_field_widget(option, &value, move |v| {
                            NetworkMessage::EditorFieldChanged {
                                key: option_name.clone(),
                                value: v,
                            }
                        })
                    })
                    .collect();

                form = form.push(
                    widget::column::with_children(fields)
                        .spacing(8)
                        .padding([0, 0, 0, 24]),
                );
            }
        }
    }

    // Custom options (not in provider definition)
    let provider_option_names: Vec<String> = provider
        .map(|p| p.options.iter().map(|o| o.name.clone()).collect())
        .unwrap_or_default();
    let mut custom_keys: Vec<String> = editor
        .options
        .keys()
        .filter(|k| {
            !provider_option_names
                .iter()
                .any(|p| p.eq_ignore_ascii_case(k))
        })
        .cloned()
        .collect();
    custom_keys.sort();

    if !custom_keys.is_empty() {
        form = form.push(widget::text::caption_heading("Additional Options"));
        let custom_rows: Vec<Element<'static, NetworkMessage>> = custom_keys
            .iter()
            .map(|key| {
                let key_for_input = key.clone();
                let key_for_remove = key.clone();
                let value = editor.options.get(key).cloned().unwrap_or_default();
                let display_key = pretty_field_name(key);
                iced_widget::row![
                    text_input("", value)
                        .label(display_key)
                        .on_input(move |v| NetworkMessage::EditorFieldChanged {
                            key: key_for_input.clone(),
                            value: v,
                        })
                        .width(Length::Fill),
                    button::standard("Remove").on_press(NetworkMessage::EditorRemoveCustomOption {
                        key: key_for_remove.clone(),
                    }),
                ]
                .spacing(6)
                .align_y(cosmic::iced::Alignment::Center)
                .into()
            })
            .collect();
        form = form.push(widget::column::with_children(custom_rows).spacing(6));
    }

    // Add custom option row
    form = form.push(widget::text::caption_heading("Add Custom Option"));
    form = form.push(
        iced_widget::row![
            text_input("Key", editor.new_option_key.clone())
                .label("Option")
                .on_input(NetworkMessage::EditorNewOptionKeyChanged)
                .width(Length::Fill),
            text_input("Value", editor.new_option_value.clone())
                .label("Value")
                .on_input(NetworkMessage::EditorNewOptionValueChanged)
                .width(Length::Fill),
            // Wrap button in a column with top padding to align with the input boxes
            // (the text_inputs have a label above them that adds ~20px)
            widget::container(
                button::standard("Add Option").on_press(NetworkMessage::EditorAddCustomOption),
            )
            .padding([20, 0, 0, 0]),
        ]
        .spacing(8)
        .align_y(cosmic::iced::Alignment::End),
    );

    form.into()
}

fn editor_view(
    state: &NetworkState,
    editor: &NetworkEditorState,
    controls_enabled: bool,
) -> Element<'static, NetworkMessage> {
    let selected_mount = state
        .selected
        .as_ref()
        .and_then(|(name, scope)| state.get_mount(name, *scope));

    let provider = rclone_provider(&editor.remote_type);

    let form = editor_form(editor, provider, controls_enabled);
    let form = bounded_form(form, 720);

    let mut layout = iced_widget::column![editor_header(editor, selected_mount, controls_enabled)]
        .spacing(16)
        .width(Length::Fill);

    if let Some(mount) = selected_mount
        && mount.is_mounted()
    {
        let mount_point = mount.config.mount_point().to_string_lossy().to_string();
        let mount_row = iced_widget::row![
            widget::text::caption("Mounted at:"),
            widget::button::link(mount_point.clone())
                .padding(0)
                .on_press(NetworkMessage::OpenMountPath(mount_point))
        ]
        .spacing(4)
        .align_y(cosmic::iced::Alignment::Center);
        layout = layout.push(mount_row);
    } else if selected_mount.is_some() {
        layout = layout.push(widget::text::caption("Not mounted"));
    }

    if !editor.is_new {
        let checked = editor.mount_on_boot.unwrap_or(false);
        let mut mount_on_boot = widget::checkbox(checked).label("Mount on boot");
        if controls_enabled && !editor.running && editor.mount_on_boot.is_some() {
            mount_on_boot = mount_on_boot.on_toggle(NetworkMessage::ToggleMountOnBoot);
        }
        layout = layout.push(mount_on_boot);
    }

    layout = layout.push(form);

    if editor.running {
        layout = layout.push(widget::text::caption("Saving configuration..."));
    }

    if let Some(error) = &editor.error {
        layout = layout.push(widget::text::caption(error.clone()));
    }

    widget::scrollable(layout)
        .width(Length::Fill)
        .height(Length::Fill)
        .apply(widget::container)
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

// ─── Wizard ──────────────────────────────────────────────────────────────────

/// Progress indicator showing dots for each step
fn wizard_progress(current: &WizardStep) -> Element<'static, NetworkMessage> {
    let current_num = current.number();
    let steps = [
        WizardStep::SelectType,
        WizardStep::NameAndScope,
        WizardStep::Connection,
        WizardStep::Authentication,
        WizardStep::Review,
    ];

    wizard_breadcrumb(
        steps
            .iter()
            .map(|step| {
                let number = step.number();
                let status = if number == current_num {
                    WizardBreadcrumbStatus::Current
                } else if number < current_num {
                    WizardBreadcrumbStatus::Completed
                } else {
                    WizardBreadcrumbStatus::Upcoming
                };

                WizardBreadcrumbStep {
                    label: step.label().to_string(),
                    status,
                    on_press: None,
                }
            })
            .collect(),
    )
}

/// Navigation buttons for wizard (Back / Next / Create)
fn wizard_nav(wizard: &NetworkWizardState) -> Element<'static, NetworkMessage> {
    let can_advance = wizard.can_advance();
    let back_message = if wizard.step != WizardStep::SelectType {
        Some(NetworkMessage::WizardBack)
    } else {
        None
    };

    let (label, action) = if wizard.step == WizardStep::Review {
        (
            "Create".to_string(),
            if can_advance {
                Some(NetworkMessage::WizardCreate)
            } else {
                None
            },
        )
    } else {
        (
            "Next".to_string(),
            if can_advance {
                Some(NetworkMessage::WizardNext)
            } else {
                None
            },
        )
    };

    wizard_step_nav(NetworkMessage::WizardCancel, back_message, label, action)
}

/// Step 1: Type selection grid
fn wizard_select_type(wizard: &NetworkWizardState) -> Element<'static, NetworkMessage> {
    let mut cards: Vec<Element<'static, NetworkMessage>> = Vec::new();

    for provider in QUICK_SETUP_PROVIDERS {
        let type_name = provider.type_name.to_string();
        let is_selected = wizard.remote_type == type_name;

        let card_content = iced_widget::column![
            provider_logo_widget(provider.type_name, 32),
            widget::text::body(provider.label.to_string())
                .font(cosmic::font::semibold())
                .width(Length::Fill)
                .align_x(cosmic::iced::alignment::Horizontal::Center),
            widget::text::caption(provider.description.to_string())
                .width(Length::Fill)
                .align_x(cosmic::iced::alignment::Horizontal::Center),
        ]
        .spacing(6)
        .align_x(cosmic::iced::Alignment::Center)
        .width(Length::Fill);

        cards.push(selectable_tile(
            card_content.into(),
            is_selected,
            Some(NetworkMessage::WizardSelectType(type_name)),
            Length::Fixed(150.0),
            Length::Fixed(120.0),
        ));
    }

    // "Advanced..." card
    let advanced_content = iced_widget::column![
        icon::from_name("preferences-other-symbolic").size(32),
        widget::text::body("Advanced...".to_string())
            .font(cosmic::font::semibold())
            .width(Length::Fill)
            .align_x(cosmic::iced::alignment::Horizontal::Center),
        widget::text::caption("All provider types".to_string())
            .width(Length::Fill)
            .align_x(cosmic::iced::alignment::Horizontal::Center),
    ]
    .spacing(6)
    .align_x(cosmic::iced::Alignment::Center)
    .width(Length::Fill);

    cards.push(selectable_tile(
        advanced_content.into(),
        false,
        Some(NetworkMessage::WizardAdvanced),
        Length::Fixed(150.0),
        Length::Fixed(120.0),
    ));

    let grid = option_tile_grid(cards);

    iced_widget::column![
        widget::text::title3("Choose a remote type"),
        widget::text::body(
            "Select a common provider below, or choose Advanced to see all available types."
                .to_string()
        ),
        widget::Space::new().height(8),
        grid,
    ]
    .spacing(8)
    .width(Length::Fill)
    .into()
}

/// Step 2: Name
fn wizard_name_scope(wizard: &NetworkWizardState) -> Element<'static, NetworkMessage> {
    let provider_label = rclone_provider(&wizard.remote_type)
        .map(|p| p.description.clone())
        .unwrap_or_else(|| wizard.remote_type.clone());

    let mut col = iced_widget::column![
        widget::text::title3("Name your remote"),
        widget::text::body(format!("Type: {provider_label}")),
        widget::Space::new().height(8),
        text_input("my-remote", wizard.name.clone())
            .label("Remote Name")
            .on_input(NetworkMessage::WizardSetName),
        widget::text::caption("Use only letters, numbers, dashes, and underscores.".to_string()),
        widget::text::caption(
            "Configurations and mount-on-login settings are stored for this desktop user."
                .to_string()
        ),
    ]
    .spacing(8)
    .width(Length::Fill)
    .max_width(500);

    if let Some(error) = &wizard.error {
        col = col.push(widget::text::caption(error.clone()));
    }

    col.into()
}

/// Step 3: Connection settings
fn wizard_connection(wizard: &NetworkWizardState) -> Element<'static, NetworkMessage> {
    let provider = rclone_provider(&wizard.remote_type);

    let mut col = iced_widget::column![
        widget::text::title3("Connection settings"),
        widget::text::body("Configure how to connect to the remote.".to_string()),
        widget::Space::new().height(8),
    ]
    .spacing(8)
    .width(Length::Fill)
    .max_width(500);

    if let Some(provider) = provider {
        let connection_fields: Vec<_> = provider
            .options
            .iter()
            .filter(|o| o.section == "connection" && !o.advanced && !o.is_hidden())
            .collect();

        if connection_fields.is_empty() {
            col = col.push(widget::text::body(
                "No connection settings required for this provider.".to_string(),
            ));
        } else {
            for option in connection_fields {
                let option_name = option.name.clone();
                let value = wizard
                    .options
                    .get(&option_name)
                    .cloned()
                    .unwrap_or_default();
                col = col.push(option_field_widget(option, &value, move |v| {
                    NetworkMessage::WizardFieldChanged {
                        key: option_name.clone(),
                        value: v,
                    }
                }));
            }
        }
    } else {
        col = col.push(widget::text::body("Unknown provider type.".to_string()));
    }

    if let Some(error) = &wizard.error {
        col = col.push(widget::text::caption(error.clone()));
    }

    col.into()
}

/// Step 4: Authentication settings
fn wizard_authentication(wizard: &NetworkWizardState) -> Element<'static, NetworkMessage> {
    let provider = rclone_provider(&wizard.remote_type);

    let mut col = iced_widget::column![
        widget::text::title3("Authentication"),
        widget::text::body("Enter credentials for the remote.".to_string()),
        widget::Space::new().height(8),
    ]
    .spacing(8)
    .width(Length::Fill)
    .max_width(500);

    if let Some(provider) = provider {
        let auth_fields: Vec<_> = provider
            .options
            .iter()
            .filter(|o| o.section == "authentication" && !o.advanced && !o.is_hidden())
            .collect();

        if auth_fields.is_empty() {
            col = col.push(widget::text::body(
                "No authentication required for this provider, or authentication is handled via OAuth.".to_string(),
            ));
        } else {
            for option in auth_fields {
                let option_name = option.name.clone();
                let value = wizard
                    .options
                    .get(&option_name)
                    .cloned()
                    .unwrap_or_default();
                col = col.push(option_field_widget(option, &value, move |v| {
                    NetworkMessage::WizardFieldChanged {
                        key: option_name.clone(),
                        value: v,
                    }
                }));
            }
        }
    } else {
        col = col.push(widget::text::body("Unknown provider type.".to_string()));
    }

    if let Some(error) = &wizard.error {
        col = col.push(widget::text::caption(error.clone()));
    }

    col.into()
}

/// Step 5: Review & Create
fn wizard_review(wizard: &NetworkWizardState) -> Element<'static, NetworkMessage> {
    let provider = rclone_provider(&wizard.remote_type);
    let provider_label = provider
        .map(|p| p.description.clone())
        .unwrap_or_else(|| wizard.remote_type.clone());

    let scope_label = match wizard.scope {
        ConfigScope::User => "User",
        ConfigScope::System => "System",
    };

    let mut col = iced_widget::column![
        widget::text::title3("Review"),
        widget::text::body("Review your remote configuration before creating it.".to_string()),
        widget::Space::new().height(8),
    ]
    .spacing(8)
    .width(Length::Fill)
    .max_width(500);

    // Summary table
    let mut summary = iced_widget::column![
        summary_row("Name", &wizard.name),
        summary_row("Type", &provider_label),
        summary_row("Scope", scope_label),
    ]
    .spacing(6);

    // Show configured options (non-empty only)
    if let Some(provider) = provider {
        for option in &provider.options {
            if let Some(value) = wizard.options.get(&option.name)
                && !value.is_empty()
            {
                let display_value = if option.is_secure() {
                    "********".to_string()
                } else {
                    value.clone()
                };
                summary = summary.push(summary_row(
                    &pretty_field_name(&option.name),
                    &display_value,
                ));
            }
        }
    }

    col = col.push(
        widget::container(summary)
            .padding(16)
            .class(cosmic::style::Container::Card),
    );

    col = col.push(widget::Space::new().height(4));
    col = col.push(widget::text::caption(
        "You can configure additional options after creating the remote.".to_string(),
    ));

    if wizard.running {
        col = col.push(widget::text::caption("Creating remote...".to_string()));
    }

    if let Some(error) = &wizard.error {
        col = col.push(widget::text::caption(error.clone()));
    }

    col.into()
}

/// Single row in the review summary
fn summary_row(label: &str, value: &str) -> Element<'static, NetworkMessage> {
    iced_widget::row![
        widget::text::body(format!("{label}:")).font(cosmic::font::semibold()),
        widget::text::body(value.to_string()),
    ]
    .spacing(8)
    .align_y(cosmic::iced::Alignment::Center)
    .into()
}

/// Full wizard view
fn wizard_view(wizard: &NetworkWizardState) -> Element<'static, NetworkMessage> {
    let step_content: Element<'static, NetworkMessage> = match wizard.step {
        WizardStep::SelectType => wizard_select_type(wizard),
        WizardStep::NameAndScope => wizard_name_scope(wizard),
        WizardStep::Connection => wizard_connection(wizard),
        WizardStep::Authentication => wizard_authentication(wizard),
        WizardStep::Review => wizard_review(wizard),
    };

    let header = iced_widget::column![
        widget::text::title2("New Remote"),
        wizard_progress(&wizard.step),
    ]
    .spacing(8)
    .width(Length::Fill);

    wizard_shell(header.into(), step_content, wizard_nav(wizard))
}

// ─── Main view router ────────────────────────────────────────────────────────

pub fn network_main_view(
    state: &NetworkState,
    controls_enabled: bool,
) -> Element<'static, NetworkMessage> {
    // Wizard takes priority over editor when present
    if let Some(wizard) = &state.wizard {
        return wizard_view(wizard);
    }

    if let Some(editor) = &state.editor {
        return editor_view(state, editor, controls_enabled);
    }

    // Empty state - no editor or wizard active
    let mut empty = iced_widget::column![
        widget::text::title2("Network Mounts"),
        widget::text::body("Select a remote from the sidebar or create a new one."),
    ]
    .spacing(10)
    .width(Length::Fill);

    if controls_enabled {
        empty =
            empty.push(button::standard("New Remote").on_press(NetworkMessage::BeginCreateRemote));
    }

    widget::container(empty)
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
