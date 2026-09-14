use storage_types::GPT_ALIGNMENT_BYTES;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiskSegmentKind {
    Partition,
    FreeSpace,
    Reserved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartitionExtent {
    pub id: usize,
    pub offset: u64,
    pub size: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiskSegment {
    pub kind: DiskSegmentKind,
    pub offset: u64,
    pub size: u64,
    pub partition_id: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum SegmentAnomaly {
    PartitionOverlapsPrevious {
        id: usize,
        partition_offset: u64,
        previous_end: u64,
    },
    PartitionStartsPastDisk {
        id: usize,
        partition_offset: u64,
        disk_size: u64,
    },
    PartitionEndPastDisk {
        id: usize,
        partition_end: u64,
        disk_size: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentComputation {
    pub segments: Vec<DiskSegment>,
    pub anomalies: Vec<SegmentAnomaly>,
}

impl DiskSegment {
    pub fn free_space(offset: u64, size: u64) -> Self {
        Self {
            kind: DiskSegmentKind::FreeSpace,
            offset,
            size,
            partition_id: None,
        }
    }

    pub fn reserved(offset: u64, size: u64) -> Self {
        Self {
            kind: DiskSegmentKind::Reserved,
            offset,
            size,
            partition_id: None,
        }
    }

    pub fn partition(id: usize, offset: u64, size: u64) -> Self {
        Self {
            kind: DiskSegmentKind::Partition,
            offset,
            size,
            partition_id: Some(id),
        }
    }
}

fn align_up(value: u64, alignment: u64) -> u64 {
    if alignment == 0 {
        return value;
    }
    value.div_ceil(alignment) * alignment
}

fn push_aligned_gap_segments(
    segments: &mut Vec<DiskSegment>,
    gap_start: u64,
    gap_end: u64,
    alignment_bytes: u64,
) {
    if gap_end <= gap_start {
        return;
    }

    let aligned_start = align_up(gap_start, alignment_bytes);

    if aligned_start >= gap_end {
        segments.push(DiskSegment::reserved(gap_start, gap_end - gap_start));
        return;
    }

    if aligned_start > gap_start {
        segments.push(DiskSegment::reserved(gap_start, aligned_start - gap_start));
    }

    segments.push(DiskSegment::free_space(
        aligned_start,
        gap_end - aligned_start,
    ));
}

pub fn compute_disk_segments(
    disk_size: u64,
    mut partitions: Vec<PartitionExtent>,
    usable_range: Option<(u64, u64)>,
) -> SegmentComputation {
    let mut anomalies = Vec::new();

    if partitions.is_empty() {
        if let Some((usable_start, usable_end)) = usable_range {
            let usable_end = usable_end.min(disk_size);
            if usable_start < usable_end {
                let mut segments = Vec::new();

                if usable_start > 0 {
                    segments.push(DiskSegment::reserved(0, usable_start));
                }

                push_aligned_gap_segments(
                    &mut segments,
                    usable_start,
                    usable_end,
                    GPT_ALIGNMENT_BYTES,
                );

                if usable_end < disk_size {
                    segments.push(DiskSegment::reserved(
                        usable_end,
                        disk_size.saturating_sub(usable_end),
                    ));
                }

                return SegmentComputation {
                    segments,
                    anomalies,
                };
            }
        }

        return SegmentComputation {
            segments: vec![DiskSegment::free_space(0, disk_size)],
            anomalies,
        };
    }

    partitions.sort_by_key(|partition| partition.offset);

    let mut segments = Vec::new();

    let (range_start, range_end, alignment_bytes) = match usable_range {
        Some((usable_start, usable_end)) if usable_start < usable_end => {
            let usable_end = usable_end.min(disk_size);
            if usable_start < usable_end {
                if usable_start > 0 {
                    segments.push(DiskSegment::reserved(0, usable_start));
                }
                (usable_start, usable_end, Some(GPT_ALIGNMENT_BYTES))
            } else {
                (0u64, disk_size, None)
            }
        }
        _ => (0u64, disk_size, None),
    };

    let mut current_offset = range_start;

    for partition in partitions {
        if partition.size == 0 {
            continue;
        }

        if partition.offset >= range_end {
            anomalies.push(SegmentAnomaly::PartitionStartsPastDisk {
                id: partition.id,
                partition_offset: partition.offset,
                disk_size: range_end,
            });

            if current_offset < range_end {
                if let Some(alignment) = alignment_bytes {
                    push_aligned_gap_segments(&mut segments, current_offset, range_end, alignment);
                } else {
                    segments.push(DiskSegment::free_space(
                        current_offset,
                        range_end.saturating_sub(current_offset),
                    ));
                }
            }
            break;
        }

        if partition.offset > current_offset {
            if let Some(alignment) = alignment_bytes {
                push_aligned_gap_segments(
                    &mut segments,
                    current_offset,
                    partition.offset,
                    alignment,
                );
            } else {
                segments.push(DiskSegment::free_space(
                    current_offset,
                    partition.offset - current_offset,
                ));
            }
            current_offset = partition.offset;
        } else if partition.offset < current_offset {
            anomalies.push(SegmentAnomaly::PartitionOverlapsPrevious {
                id: partition.id,
                partition_offset: partition.offset,
                previous_end: current_offset,
            });
        }

        let partition_end = partition.offset.saturating_add(partition.size);
        let effective_end = if partition_end > range_end {
            anomalies.push(SegmentAnomaly::PartitionEndPastDisk {
                id: partition.id,
                partition_end,
                disk_size: range_end,
            });
            range_end
        } else {
            partition_end
        };

        // If the partition overlaps, clamp its visible start to keep the output ordered and
        // non-overlapping for UI rendering.
        let effective_offset = current_offset.max(partition.offset);
        let effective_size = effective_end.saturating_sub(effective_offset);

        if effective_size > 0 {
            segments.push(DiskSegment::partition(
                partition.id,
                effective_offset,
                effective_size,
            ));
            current_offset = effective_offset.saturating_add(effective_size);
        }

        if current_offset >= range_end {
            break;
        }
    }

    if current_offset < range_end {
        if let Some(alignment) = alignment_bytes {
            push_aligned_gap_segments(&mut segments, current_offset, range_end, alignment);
        } else {
            segments.push(DiskSegment::free_space(
                current_offset,
                range_end.saturating_sub(current_offset),
            ));
        }
    }

    if range_end < disk_size {
        segments.push(DiskSegment::reserved(
            range_end,
            disk_size.saturating_sub(range_end),
        ));
    }

    SegmentComputation {
        segments,
        anomalies,
    }
}

#[path = "../../tests/unit/utils/segments_tests.rs"]
#[cfg(test)]
mod tests;
