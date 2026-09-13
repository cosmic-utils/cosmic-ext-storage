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
    let res = compute_disk_segments(10 * 1024 * 1024, vec![], Some((34 * 512, 10 * 1024 * 1024)));
    assert!(
        res.segments
            .iter()
            .any(|s| s.kind == DiskSegmentKind::Reserved)
    );
    assert!(
        res.segments
            .iter()
            .all(|s| s.kind != DiskSegmentKind::FreeSpace || s.offset % GPT_ALIGNMENT_BYTES == 0)
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
