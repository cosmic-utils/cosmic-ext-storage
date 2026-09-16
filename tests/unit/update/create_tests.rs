use super::*;
use crate::state::dialogs::CreatePartitionDialog;
use rstest::{fixture, rstest};
use storage_types::FilesystemToolInfo;

#[fixture]
fn create_dialog() -> CreatePartitionDialog {
    CreatePartitionDialog {
        operation_id: None,
        info: CreatePartitionInfo {
            table_type: "gpt".into(),
            selected_partition_type_index: storage_types::COMMON_GPT_TYPES
                .iter()
                .position(|kind| kind.filesystem_type == "ext4")
                .unwrap(),
            size: 1024,
            max_size: 2048,
            ..Default::default()
        },
        step: CreatePartitionStep::Basics,
        running: false,
        error: None,
        filesystem_tools: vec![FilesystemToolInfo {
            fs_type: "ext4".into(),
            fs_name: "EXT4".into(),
            command: "mkfs.ext4".into(),
            package_hint: "e2fsprogs".into(),
            available: true,
        }],
    }
}

#[rstest]
#[case::available(true, "gpt", false, true)]
#[case::unavailable(false, "gpt", false, false)]
#[case::unknown_table(true, "unknown", false, false)]
#[case::stale_index(true, "gpt", true, false)]
fn basics_requires_a_known_available_filesystem(
    mut create_dialog: CreatePartitionDialog,
    #[case] available: bool,
    #[case] table: &str,
    #[case] stale_index: bool,
    #[case] expected: bool,
) {
    create_dialog.filesystem_tools[0].available = available;
    create_dialog.info.table_type = table.into();
    if stale_index {
        create_dialog.info.selected_partition_type_index = usize::MAX;
    }
    assert_eq!(create_partition_step_can_advance(&create_dialog), expected);
    // An unrelated installed tool must not enable the selected filesystem.
    create_dialog.filesystem_tools[0].fs_type = "unrelated".into();
    assert!(!create_partition_step_can_advance(&create_dialog));
    create_dialog.filesystem_tools.clear();
    assert!(!create_partition_step_can_advance(&create_dialog));
}

#[rstest]
#[case::zero(0, false)]
#[case::minimum(1, true)]
#[case::maximum(2048, true)]
#[case::oversized(2049, false)]
#[case::overflow_boundary(u64::MAX, false)]
fn sizing_requires_a_nonzero_size_within_the_extent(
    mut create_dialog: CreatePartitionDialog,
    #[case] size: u64,
    #[case] expected: bool,
) {
    create_dialog.step = CreatePartitionStep::Sizing;
    create_dialog.info.size = size;
    assert_eq!(create_partition_step_can_advance(&create_dialog), expected);
}

#[rstest]
fn final_step_does_not_reapply_basics_or_sizing_navigation_checks(
    mut create_dialog: CreatePartitionDialog,
) {
    create_dialog.step = CreatePartitionStep::Options;
    create_dialog.filesystem_tools.clear();
    create_dialog.info.size = 0;
    // Submission validation is separate; this helper only controls navigation.
    assert!(create_partition_step_can_advance(&create_dialog));
}
