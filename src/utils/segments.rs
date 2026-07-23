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

#[cfg(test)]
mod tests {
    use super::*;

    fn part(id: usize, offset: u64, size: u64) -> PartitionExtent {
        PartitionExtent { id, offset, size }
    }

    #[test]
    fn usable_range_reserves_start_region() {
        let disk_size = 10 * GPT_ALIGNMENT_BYTES;
        let res = compute_disk_segments(disk_size, vec![], Some((GPT_ALIGNMENT_BYTES, disk_size)));

        assert_eq!(
            res.segments,
            vec![
                DiskSegment::reserved(0, GPT_ALIGNMENT_BYTES),
                DiskSegment::free_space(GPT_ALIGNMENT_BYTES, disk_size - GPT_ALIGNMENT_BYTES)
            ]
        );
    }

    #[test]
    fn empty_partitions_is_single_free_space() {
        let res = compute_disk_segments(1000, vec![], None);
        assert_eq!(res.segments, vec![DiskSegment::free_space(0, 1000)]);
        assert!(res.anomalies.is_empty());
    }

    #[test]
    fn single_partition_with_trailing_free() {
        let res = compute_disk_segments(1000, vec![part(0, 100, 200)], None);
        assert_eq!(
            res.segments,
            vec![
                DiskSegment::free_space(0, 100),
                DiskSegment::partition(0, 100, 200),
                DiskSegment::free_space(300, 700)
            ]
        );
    }

    #[test]
    fn multiple_partitions_with_gaps_and_unsorted_input() {
        let res = compute_disk_segments(1000, vec![part(1, 600, 100), part(0, 100, 200)], None);
        assert_eq!(
            res.segments,
            vec![
                DiskSegment::free_space(0, 100),
                DiskSegment::partition(0, 100, 200),
                DiskSegment::free_space(300, 300),
                DiskSegment::partition(1, 600, 100),
                DiskSegment::free_space(700, 300)
            ]
        );
    }

    #[test]
    fn overlapping_partitions_do_not_panic_and_remain_ordered() {
        let res = compute_disk_segments(1000, vec![part(0, 100, 300), part(1, 200, 200)], None);
        assert!(
            res.anomalies
                .iter()
                .any(|a| matches!(a, SegmentAnomaly::PartitionOverlapsPrevious { id: 1, .. }))
        );

        // Layout remains ordered and non-overlapping for UI rendering.
        let segments = res.segments;
        for w in segments.windows(2) {
            let a = w[0];
            let b = w[1];
            assert!(a.offset.saturating_add(a.size) <= b.offset);
        }
    }

    #[test]
    fn partition_end_past_disk_is_clamped() {
        let res = compute_disk_segments(1000, vec![part(0, 900, 200)], None);
        assert!(
            res.anomalies
                .iter()
                .any(|a| matches!(a, SegmentAnomaly::PartitionEndPastDisk { id: 0, .. }))
        );
        assert_eq!(
            res.segments,
            vec![
                DiskSegment::free_space(0, 900),
                DiskSegment::partition(0, 900, 100)
            ]
        );
    }

    #[test]
    fn extremely_small_partitions_are_preserved_as_non_zero_size() {
        let res = compute_disk_segments(1000, vec![part(0, 10, 1)], None);
        assert_eq!(
            res.segments,
            vec![
                DiskSegment::free_space(0, 10),
                DiskSegment::partition(0, 10, 1),
                DiskSegment::free_space(11, 989)
            ]
        );
    }

    #[test]
    fn gpt_reserved_and_alignment_padding_is_not_free_space() {
        // Usable range begins before 1MiB alignment.
        let res =
            compute_disk_segments(10 * 1024 * 1024, vec![], Some((34 * 512, 10 * 1024 * 1024)));
        assert!(
            res.segments
                .iter()
                .any(|s| s.kind == DiskSegmentKind::Reserved)
        );
        assert!(
            res.segments.iter().all(
                |s| s.kind != DiskSegmentKind::FreeSpace || s.offset % GPT_ALIGNMENT_BYTES == 0
            )
        );
    }

    #[test]
    fn gpt_reserves_trailing_space_when_usable_end_less_than_disk_size() {
        // Test with usable_end < disk_size to verify trailing reserved space is correctly handled
        let disk_size = 10 * 1024 * 1024; // 10 MiB
        let usable_start = GPT_ALIGNMENT_BYTES; // 1 MiB
        let usable_end = 9 * 1024 * 1024; // 9 MiB (leaving 1 MiB reserved at the end)

        let res = compute_disk_segments(disk_size, vec![], Some((usable_start, usable_end)));

        // Should have reserved space at both start and end
        let reserved_segments: Vec<_> = res
            .segments
            .iter()
            .filter(|s| s.kind == DiskSegmentKind::Reserved)
            .collect();

        assert!(
            !reserved_segments.is_empty(),
            "Should have at least one reserved segment"
        );

        // Verify there's a reserved segment at the end
        let has_end_reserved = res
            .segments
            .iter()
            .any(|s| s.kind == DiskSegmentKind::Reserved && s.offset + s.size == disk_size);
        assert!(
            has_end_reserved,
            "Should have reserved segment at the end of disk"
        );

        // All free space should be aligned and within usable range
        for seg in &res.segments {
            if seg.kind == DiskSegmentKind::FreeSpace {
                assert!(
                    seg.offset % GPT_ALIGNMENT_BYTES == 0,
                    "Free space should be aligned"
                );
                assert!(
                    seg.offset >= usable_start,
                    "Free space should start at or after usable_start"
                );
                assert!(
                    seg.offset + seg.size <= usable_end,
                    "Free space should end at or before usable_end"
                );
            }
        }
    }
}
