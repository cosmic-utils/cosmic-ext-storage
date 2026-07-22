use cosmic::Task;

use crate::app::Message;
use crate::state::btrfs::BtrfsState;
use crate::state::dialogs::ShowDialog;

use crate::state::volumes::DetailTab;
use crate::state::volumes::VolumesControl;

fn maybe_initialize_btrfs_state_for_segment(
    control: &mut VolumesControl,
    segment_index: usize,
    sidebar_device_path: &str,
    log_context: &str,
) -> Option<Task<cosmic::Action<Message>>> {
    if let Some(segment) = control.segments.get(segment_index)
        && let Some(volume) = &segment.volume
    {
        let btrfs_info = crate::state::btrfs::detect_btrfs_for_volume(&control.volumes, volume);
        tracing::info!(
            "{}: segment_index={}, btrfs_detected={}, id_type={}, has_filesystem={}, mount_points={:?}",
            log_context,
            segment_index,
            btrfs_info.is_some(),
            volume.id_type,
            volume.has_filesystem,
            volume.mount_points
        );

        if let Some((mount_point, block_path)) = btrfs_info {
            tracing::info!(
                "{}: Initializing BTRFS state with mount_point={:?}, block_path={}",
                log_context,
                mount_point,
                block_path
            );
            control.btrfs_state = Some(BtrfsState::new(
                mount_point.clone(),
                Some(block_path.clone()),
            ));

            if let Some(mp) = mount_point {
                let tasks = vec![
                    Task::done(cosmic::Action::App(Message::SidebarSelectChild {
                        device_path: sidebar_device_path.to_string(),
                    })),
                    Task::done(cosmic::Action::App(Message::BtrfsLoadSubvolumes {
                        block_path: block_path.clone(),
                        mount_point: mp.clone(),
                    })),
                    Task::done(cosmic::Action::App(Message::BtrfsLoadUsage {
                        block_path,
                        mount_point: mp,
                    })),
                ];

                return Some(Task::batch(tasks));
            }
        } else {
            control.btrfs_state = None;
        }
    }

    None
}

pub(super) fn segment_selected(
    control: &mut VolumesControl,
    index: usize,
    dialog: &Option<ShowDialog>,
) -> Task<cosmic::Action<Message>> {
    if dialog.is_none() {
        let Some(last_index) = control.segments.len().checked_sub(1) else {
            control.selected_segment = 0;
            control.selected_volume = None;
            return Task::batch(vec![Task::done(cosmic::Action::App(
                Message::SidebarClearChildSelection,
            ))]);
        };

        let index = index.min(last_index);
        control.selected_segment = index;
        control.selected_volume = None;
        control.detail_tab = DetailTab::VolumeInfo;
        control.segments.iter_mut().for_each(|s| s.state = false);
        if let Some(segment) = control.segments.get_mut(index) {
            segment.state = true;
        }

        let sidebar_device_path = control
            .segments
            .get(index)
            .and_then(|segment| segment.volume.as_ref())
            .map(|volume| {
                volume
                    .device_path
                    .clone()
                    .unwrap_or_else(|| volume.label.clone())
            })
            .unwrap_or_default();

        if let Some(task) = maybe_initialize_btrfs_state_for_segment(
            control,
            index,
            &sidebar_device_path,
            "segment_selected",
        ) {
            return task;
        }

        // Sync with sidebar: select the segment's volume node if it has one
        if let Some(segment) = control.segments.get(index)
            && let Some(vol) = &segment.volume
        {
            let device_path = vol.device_path.clone().unwrap_or_else(|| vol.label.clone());
            return Task::batch(vec![Task::done(cosmic::Action::App(
                Message::SidebarSelectChild { device_path },
            ))]);
        }

        // No volume on this segment (e.g., free space), clear selection
        return Task::batch(vec![Task::done(cosmic::Action::App(
            Message::SidebarClearChildSelection,
        ))]);
    }

    Task::none()
}

pub(super) fn select_volume(
    control: &mut VolumesControl,
    segment_index: usize,
    device_path: String,
    dialog: &Option<ShowDialog>,
) -> Task<cosmic::Action<Message>> {
    if dialog.is_none() {
        let Some(last_index) = control.segments.len().checked_sub(1) else {
            control.selected_segment = 0;
            control.selected_volume = None;
            return Task::none();
        };

        let segment_index = segment_index.min(last_index);
        control.selected_segment = segment_index;
        control.selected_volume = Some(device_path.clone());
        control.detail_tab = DetailTab::VolumeInfo;
        control.segments.iter_mut().for_each(|s| s.state = false);
        if let Some(segment) = control.segments.get_mut(segment_index) {
            segment.state = true;
        }

        if let Some(task) = maybe_initialize_btrfs_state_for_segment(
            control,
            segment_index,
            &device_path,
            "select_volume",
        ) {
            return task;
        }

        // Sync with sidebar: select the corresponding volume in sidebar
        return Task::batch(vec![Task::done(cosmic::Action::App(
            Message::SidebarSelectChild { device_path },
        ))]);
    }

    Task::none()
}
