use crate::models::{UiDrive, UiVolume};
use crate::{
    fl,
    state::btrfs::BtrfsState,
    utils::{DiskSegmentKind, PartitionExtent, SegmentAnomaly, compute_disk_segments},
};
use storage_types::{
    ByteRange, CreatePartitionInfo, FilesystemToolInfo, PartitionInfo, UsageCategory,
    UsageScanParallelismPreset, UsageScanResult, VolumeInfo,
};

/// Which detail tab is active below the drive header
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DetailTab {
    #[default]
    VolumeInfo,
    Usage,
    BtrfsManagement,
}

#[derive(Debug, Clone)]
pub struct UsageTabState {
    pub loading: bool,
    pub progress_processed_bytes: u64,
    pub progress_estimated_total_bytes: u64,
    pub active_scan_id: Option<String>,
    pub result: Option<UsageScanResult>,
    pub selected_categories: Vec<UsageCategory>,
    pub show_all_files: bool,
    pub show_all_files_authorized_for_session: bool,
    pub top_files_per_category: u32,
    pub selected_paths: Vec<String>,
    pub selection_anchor_index: Option<usize>,
    pub selection_modifiers: cosmic::iced::keyboard::Modifiers,
    pub deleting: bool,
    pub error: Option<String>,
    pub operation_status: Option<String>,
    pub wizard_open: bool,
    pub wizard_loading_mounts: bool,
    pub wizard_mount_points: Vec<String>,
    pub wizard_selected_mount_points: Vec<String>,
    pub wizard_show_all_files: bool,
    pub wizard_parallelism_preset: UsageScanParallelismPreset,
    pub wizard_error: Option<String>,
    pub scan_mount_points: Vec<String>,
    pub scan_parallelism_preset: UsageScanParallelismPreset,
}

impl Default for UsageTabState {
    fn default() -> Self {
        Self {
            loading: false,
            progress_processed_bytes: 0,
            progress_estimated_total_bytes: 0,
            active_scan_id: None,
            result: None,
            selected_categories: UsageCategory::ALL.to_vec(),
            show_all_files: false,
            show_all_files_authorized_for_session: false,
            top_files_per_category: 20,
            selected_paths: Vec::new(),
            selection_anchor_index: None,
            selection_modifiers: cosmic::iced::keyboard::Modifiers::default(),
            deleting: false,
            error: None,
            operation_status: None,
            wizard_open: false,
            wizard_loading_mounts: false,
            wizard_mount_points: Vec::new(),
            wizard_selected_mount_points: Vec::new(),
            wizard_show_all_files: false,
            wizard_parallelism_preset: UsageScanParallelismPreset::Balanced,
            wizard_error: None,
            scan_mount_points: Vec::new(),
            scan_parallelism_preset: UsageScanParallelismPreset::Balanced,
        }
    }
}

pub struct VolumesControl {
    pub selected_segment: usize,
    pub selected_volume: Option<String>,
    pub segments: Vec<Segment>,
    pub show_reserved: bool,
    /// Drive device path
    pub device: String,
    /// Drive size in bytes
    pub size: u64,
    /// Partition table type
    pub partition_table_type: Option<String>,
    /// GPT usable range
    pub gpt_usable_range: Option<ByteRange>,
    /// Flat partition list (for segment computation)
    pub partitions: Vec<PartitionInfo>,
    /// Hierarchical volume tree (for BTRFS detection)
    pub volumes: Vec<UiVolume>,
    /// BTRFS management state for the currently selected volume (if BTRFS)
    pub btrfs_state: Option<BtrfsState>,
    /// Which detail tab is currently displayed
    pub detail_tab: DetailTab,
    /// Cached filesystem tool availability from service
    pub filesystem_tools: Vec<FilesystemToolInfo>,
    /// Usage tab state for global categorized usage scan
    pub usage_state: UsageTabState,
}

#[derive(Clone, Debug)]
pub struct Segment {
    pub label: String,
    pub name: String,
    #[allow(dead_code)]
    pub partition_type: String,
    pub size: u64,
    pub offset: u64,
    pub state: bool,
    pub kind: DiskSegmentKind,
    pub width: u16,
    pub volume: Option<VolumeInfo>,
    pub device_path: Option<String>, // Device path to look up PartitionInfo
    pub table_type: String,
}

#[derive(Copy, Clone)]
pub enum ToggleState {
    Normal,
    Active,
    Disabled,
    Hovered,
    Pressed,
}

impl ToggleState {
    pub fn active_or(selected: &bool, toggle: ToggleState) -> Self {
        if *selected {
            ToggleState::Active
        } else {
            toggle
        }
    }
}

impl Segment {
    pub fn free_space(offset: u64, size: u64, table_type: String) -> Self {
        Self {
            label: fl!("free-space-segment"),
            name: "".into(),
            partition_type: "".into(),
            size,
            offset,
            state: false,
            kind: DiskSegmentKind::FreeSpace,
            width: 0,
            volume: None,
            device_path: None,
            table_type,
        }
    }

    pub fn reserved(offset: u64, size: u64, table_type: String) -> Self {
        Self {
            label: fl!("reserved-space-segment"),
            name: "".into(),
            partition_type: "".into(),
            size,
            offset,
            state: false,
            kind: DiskSegmentKind::Reserved,
            width: 0,
            volume: None,
            device_path: None,
            table_type,
        }
    }

    pub fn get_create_info(&self) -> CreatePartitionInfo {
        // Auto-select appropriate unit and format size text
        let unit = crate::utils::SizeUnit::auto_select(self.size);
        let size_value = unit.from_bytes(self.size);

        CreatePartitionInfo {
            max_size: self.size,
            offset: self.offset,
            size: self.size,
            table_type: self.table_type.clone(),
            size_text: format!("{:.2}", size_value),
            size_unit_index: unit.to_index(),
            ..Default::default()
        }
    }

    pub fn new(partition: &PartitionInfo, volume_info: Option<VolumeInfo>) -> Self {
        let mut name = partition.name.clone();
        if name.is_empty() {
            name = fl!("filesystem");
        }

        let mut type_str = volume_info
            .as_ref()
            .map(|v| v.id_type.to_uppercase())
            .unwrap_or_default();
        type_str = format!("{} - {}", type_str, partition.type_name);

        Self {
            label: name,
            name: partition.device.clone(),
            partition_type: type_str,
            size: partition.size,
            offset: partition.offset,
            state: false,
            kind: DiskSegmentKind::Partition,
            width: 0,
            volume: volume_info,
            device_path: Some(partition.device.clone()),
            table_type: partition.table_type.clone(),
        }
    }

    pub fn get_segments(
        size: u64,
        partition_table_type: &Option<String>,
        gpt_usable_range: Option<ByteRange>,
        partitions: &[PartitionInfo],
        all_volumes: &[VolumeInfo],
        show_reserved: bool,
    ) -> Vec<Segment> {
        let table_type = partition_table_type.clone().unwrap_or_default();
        const DOS_RESERVED_START_BYTES: u64 = 1024 * 1024;

        let usable_range = match table_type.as_str() {
            "gpt" => gpt_usable_range.map(|r| (r.start, r.end)),
            "dos" => {
                if size > DOS_RESERVED_START_BYTES {
                    Some((DOS_RESERVED_START_BYTES, size))
                } else {
                    None
                }
            }
            _ => None,
        };

        let extents: Vec<PartitionExtent> = partitions
            .iter()
            .enumerate()
            .map(|(id, p)| PartitionExtent {
                id,
                offset: p.offset,
                size: p.size,
            })
            .collect();

        let computation = compute_disk_segments(size, extents, usable_range);
        for anomaly in computation.anomalies {
            match anomaly {
                SegmentAnomaly::PartitionOverlapsPrevious {
                    id,
                    partition_offset,
                    previous_end,
                } => {
                    tracing::warn!(
                        id,
                        partition_offset,
                        previous_end,
                        "partition segmentation anomaly: overlaps previous segment"
                    );
                }
                SegmentAnomaly::PartitionStartsPastDisk {
                    id,
                    partition_offset,
                    disk_size,
                } => {
                    tracing::warn!(
                        id,
                        partition_offset,
                        disk_size,
                        "partition segmentation anomaly: starts past disk end"
                    );
                }
                SegmentAnomaly::PartitionEndPastDisk {
                    id,
                    partition_end,
                    disk_size,
                } => {
                    tracing::warn!(
                        id,
                        partition_end,
                        disk_size,
                        "partition segmentation anomaly: ends past disk end"
                    );
                }
            }
        }

        let mut segments: Vec<Segment> = Vec::new();
        for seg in computation.segments {
            match seg.kind {
                DiskSegmentKind::FreeSpace => {
                    segments.push(Segment::free_space(
                        seg.offset,
                        seg.size,
                        table_type.clone(),
                    ));
                }
                DiskSegmentKind::Reserved => {
                    segments.push(Segment::reserved(seg.offset, seg.size, table_type.clone()));
                }
                DiskSegmentKind::Partition => {
                    let Some(partition_id) = seg.partition_id else {
                        continue;
                    };
                    let Some(p) = partitions.get(partition_id) else {
                        continue;
                    };

                    // Find corresponding volume info by device path
                    let volume_info = all_volumes
                        .iter()
                        .find(|v| v.device_path.as_ref() == Some(&p.device))
                        .cloned();

                    let mut s = Segment::new(p, volume_info);
                    // Use computed extents so clamping (e.g., end-past-disk) is reflected.
                    s.offset = seg.offset;
                    s.size = seg.size;
                    segments.push(s);
                }
            }
        }

        // Convert tiny free space (<10MB) to reserved space to hide alignment/reserved gaps,
        // UNLESS the whole drive is free space OR the drive is very small (<100MB).
        const TINY_FREE_THRESHOLD: u64 = 10 * 1024 * 1024; // 10MB
        const SMALL_DRIVE_THRESHOLD: u64 = 100 * 1024 * 1024; // 100MB

        let all_free_space = segments
            .iter()
            .all(|s| s.kind == DiskSegmentKind::FreeSpace);
        let is_small_drive = size < SMALL_DRIVE_THRESHOLD;

        if !all_free_space && !is_small_drive {
            for segment in segments.iter_mut() {
                if segment.kind == DiskSegmentKind::FreeSpace && segment.size < TINY_FREE_THRESHOLD
                {
                    segment.kind = DiskSegmentKind::Reserved;
                }
            }
        }

        if segments.len() > 1 {
            let mut merged: Vec<Segment> = Vec::with_capacity(segments.len());
            for segment in segments.into_iter() {
                if let Some(last) = merged.last_mut()
                    && last.kind == DiskSegmentKind::Reserved
                    && segment.kind == DiskSegmentKind::Reserved
                    && last.offset.saturating_add(last.size) == segment.offset
                {
                    last.size = last.size.saturating_add(segment.size);
                } else {
                    merged.push(segment);
                }
            }
            segments = merged;
        }

        if !show_reserved {
            segments.retain(|s| {
                if s.kind == DiskSegmentKind::Reserved {
                    return false;
                }
                if s.kind == DiskSegmentKind::FreeSpace && s.size < 1048576 {
                    return false;
                }
                true
            });

            // Ensure the UI always has at least one segment to render/select.
            if segments.is_empty() && size > 0 {
                if let Some((start, end)) = usable_range {
                    if end > start {
                        segments.push(Segment::free_space(
                            start,
                            end.saturating_sub(start),
                            table_type.clone(),
                        ));
                    } else {
                        segments.push(Segment::free_space(0, size, table_type.clone()));
                    }
                } else {
                    segments.push(Segment::free_space(0, size, table_type.clone()));
                }
            }
        }

        // Figure out Portion value (based on what we're showing).
        let visible_total = segments.iter().map(|s| s.size).sum::<u64>();
        let denom = visible_total.max(1);
        segments.iter_mut().for_each(|s| {
            s.width = (((s.size as f64 / denom as f64) * 1000.).log10().ceil() as u16).max(1);
        });

        segments
    }
}

pub(crate) fn find_volume_in_ui_tree<'a>(
    volumes: &'a [UiVolume],
    device_path: &str,
) -> Option<&'a UiVolume> {
    for v in volumes {
        if v.device() == Some(device_path) {
            return Some(v);
        }
        if let Some(child) = find_volume_in_ui_tree(&v.children, device_path) {
            return Some(child);
        }
    }
    None
}

pub(crate) fn find_volume_for_partition<'a>(
    volumes: &'a [UiVolume],
    partition_volume: &VolumeInfo,
) -> Option<&'a UiVolume> {
    let Some(target) = &partition_volume.device_path else {
        return None;
    };
    find_volume_in_ui_tree(volumes, target)
}

pub(crate) fn find_segment_for_volume(
    volumes_control: &VolumesControl,
    device_path: &str,
) -> Option<(usize, bool)> {
    for (segment_idx, segment) in volumes_control.segments.iter().enumerate() {
        let Some(segment_vol) = &segment.volume else {
            continue;
        };

        if segment_vol
            .device_path
            .as_ref()
            .is_some_and(|p| p == device_path)
        {
            return Some((segment_idx, false));
        }

        let Some(segment_device) = &segment_vol.device_path else {
            continue;
        };

        let segment_node = volumes_control
            .volumes
            .iter()
            .find(|v| v.device() == Some(segment_device.as_str()));

        let Some(segment_node) = segment_node else {
            continue;
        };

        if find_volume_in_ui_tree(&segment_node.children, device_path).is_some() {
            return Some((segment_idx, true));
        }
    }

    None
}

impl VolumesControl {
    pub fn new(
        drive: &UiDrive,
        show_reserved: bool,
        filesystem_tools: Vec<FilesystemToolInfo>,
    ) -> Self {
        // Flatten all volumes to a list for lookup
        fn flatten_volumes(node: &UiVolume, out: &mut Vec<VolumeInfo>) {
            out.push(node.volume.clone());
            for child in &node.children {
                flatten_volumes(child, out);
            }
        }

        let mut all_volumes = Vec::new();
        for root in &drive.volumes {
            flatten_volumes(root, &mut all_volumes);
        }

        let mut segments: Vec<Segment> = Segment::get_segments(
            drive.disk.size,
            &drive.disk.partition_table_type,
            drive.disk.gpt_usable_range,
            &drive.partitions,
            &all_volumes,
            show_reserved,
        );
        if let Some(first) = segments.first_mut() {
            first.state = true;
        }

        Self {
            device: drive.device().to_string(),
            size: drive.disk.size,
            partition_table_type: drive.disk.partition_table_type.clone(),
            gpt_usable_range: drive.disk.gpt_usable_range,
            partitions: drive.partitions.clone(),
            volumes: drive.volumes.clone(),
            selected_segment: 0,
            selected_volume: None,
            segments,
            show_reserved,
            btrfs_state: None,
            detail_tab: DetailTab::default(),
            filesystem_tools,
            usage_state: UsageTabState::default(),
        }
    }

    pub fn selected_volume_node(&self) -> Option<&UiVolume> {
        let device_path = self.selected_volume.as_deref()?;

        // Recursively search for volume by device path
        fn find_in_tree<'a>(volumes: &'a [UiVolume], device: &str) -> Option<&'a UiVolume> {
            for vol in volumes {
                if vol.device() == Some(device) {
                    return Some(vol);
                }
                if let Some(found) = find_in_tree(&vol.children, device) {
                    return Some(found);
                }
            }
            None
        }

        find_in_tree(&self.volumes, device_path)
    }

    pub fn set_show_reserved(&mut self, show_reserved: bool) {
        if self.show_reserved == show_reserved {
            return;
        }

        self.show_reserved = show_reserved;

        // Flatten volumes again
        fn flatten_volumes(node: &UiVolume, out: &mut Vec<VolumeInfo>) {
            out.push(node.volume.clone());
            for child in &node.children {
                flatten_volumes(child, out);
            }
        }

        let mut all_volumes = Vec::new();
        for root in &self.volumes {
            flatten_volumes(root, &mut all_volumes);
        }

        self.segments = Segment::get_segments(
            self.size,
            &self.partition_table_type,
            self.gpt_usable_range,
            &self.partitions,
            &all_volumes,
            self.show_reserved,
        );
        self.selected_segment = 0;
        self.selected_volume = None;
        self.btrfs_state = None;
        self.usage_state = UsageTabState::default();
        if let Some(first) = self.segments.first_mut() {
            first.state = true;
        }
    }
}
