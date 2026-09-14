# rstest migration assertion map

Status: host discovery, native execution and source-aligned coverage preservation verified.

Original source bodies are preserved in
`target/rstest-adoption/baseline/test-sources.tar.gz`; original package/target/
test/ignored identities in `baseline/nextest.json`. Paths below are relative to
the repository; this document describes assertions, not a second runner inventory.

| Original test/group | Required preserved assertions / intended replacement |
| --- | --- |
| Shutdown `quarantine_requires_exact_ordered_stack_after_close` | Known ordered stack passes narrowly; no close, wrong signal/kind/schema, missing/reversed frames, wrong library/owner and conflicting exit each fail; malformed JSON remains rejected. Nine named mutations plus baseline/malformed checks. |
| Shutdown `changed_scope_expiry_and_supervisor_failure_are_not_quarantined` | Missing supervisor status, wrong case, case hash, environment hash, lock hash and expiry each fail; policy ID and SHA length retained separately. |
| Shutdown `functional_failure_cannot_be_overridden_by_shutdown_status` | All four outcomes with functional failure fail; functional success with failed/not-attempted fails, known crash has qualified status, clean succeeds. Eight named rows. |
| Shutdown `clean_shutdown_requires_matching_successful_process_evidence` | Matching clean status succeeds, both conflicting process status directions fail. Retain. |
| UI `executable_cases_require_actions_postconditions_hashes_and_closed_fields` | Valid reload identity, wrong schema/timeout/action, duplicate step ID, unknown field and empty program. Independent named cases; unchanged program semantics. |
| UI `exact_selectors_reject_ambiguous_nodes_and_support_ancestor_scoping` | Ambiguity, ancestor-scoped success, absent state, case-sensitive name, missing selector identity and automation ID fallback. Retain coherent tree sequence. |
| UI `semantic_assertions_require_observed_state_and_exact_focus` | Preserve deliberately mutated tree/count/focus state in place; four valid property pairs, unknown property and wrong value move to six fresh-tree named cases. |
| UI `keyboard_and_fixture_inputs_cannot_inject_arguments_or_escape_root` | All 13 valid and four invalid keys; local file allowed, parent/absolute/symlink escape denied. Separate named tables with fresh owned filesystem state. |
| UI `shutdown_wait_preserves_nonzero_exit_and_times_out` | Exit 7, deadline error, explicit child kill and reap. Retain. |
| Size `test_bytes_identity`, MB/GB conversion tests, `test_terabytes_conversion` | Preserve 12345-byte identity, 100 MB, 5 GB, 1 GB and 2.5 TB round trip with existing tolerances. Named conversions/round trip; identity stays explicit. |
| Size `test_unit_index_roundtrip`, `test_auto_select`, `test_labels` | Five units round-trip; 512/2048/5 MiB/3 GiB/2 TiB select expected units; five ordered labels. Named rows, label-order test retained. |
| Backend `schema`, `physical_state`, `logical_network_state`, `workflow_state`, `contract_surface` | Original function identities and all semantic bodies retained; setup/path/owned directory changes only. Nonstandard overlay/secrets/backend-constructor checks stay explicit. |
| Backend control unit tests | Original identities/authentication/checkpoint/serialization/redaction assertions retained; shared path/setup and fresh socket root only. |
| Root `ui_scenario_contract`, `scenario_control_runtime` | Original identities, feature gating and semantic assertions retained; shared setup and fresh socket/token root only. |
| Root `application_workflows` | Original identities and bodies retained. Owned harness injected; intentional dual-world and success/failure setup stays explicit where clearer. |
| Native `partitions`, `filesystems`, `images`, `encryption`, `btrfs`, `logical` | Whole existing lifecycle bodies and test identities retained; existing label/size passed to awaited owned lab fixture. No independent lifecycle fragments. |
| Outer bridge 15 forwarding wrappers | One named target/exact-filter case per original wrapper; no extra Cartesian combinations. Dedicated deliberate failure and host-safe cleanup regression retained. |

Other tests are unchanged except any exact-identity references required by these
conversions. Additional fixture/selection regressions are new behavior coverage,
not substitutes for old assertions or the eight mandatory UI cases.

## Actual discovery audit

Captured with workspace/all-features Cargo `-- --list` and Nextest JSON listing.
Non-parameterized fixture migrations preserve their old exact identities.
The bridge generated names have **zero-padded indexes**, verified below rather
than guessed. Inner native names are unchanged.

### Bridge wrapper map

| Old outer test | Discovered new outer case | Inner target / exact filter |
| --- | --- | --- |
| `capability_runs_in_the_private_storage_lab` | `native_case::case_01_capability` | `capability` / `capability_starts_private_dbus_udisks_and_sftp` |
| `partition_table_runs_in_the_private_storage_lab` | `native_case::case_02_partition_table` | `capability` / `partition_table_round_trip_uses_the_private_adapter_transport` |
| `fixture_drop_cleanup_runs_in_the_private_storage_lab` | `native_case::case_03_fixture_drop_cleanup` | `capability` / `dropping_a_fixture_detaches_its_ledgered_loop` |
| `filesystem_runs_in_the_private_storage_lab` | `native_case::case_04_filesystem` | `capability` / `filesystem_format_and_label_use_the_production_adapter` |
| `luks_runs_in_the_private_storage_lab` | `native_case::case_05_luks` | `capability` / `luks_unlock_rejects_bad_secret_and_locks_cleanly` |
| `failed_case_cleanup_runs_in_the_private_storage_lab` | `native_case::case_06_failed_case_cleanup` | `capability` / `failing_case_still_removes_all_ledgered_resources` |
| `partition_lifecycle_runs_in_the_private_storage_lab` | `native_case::case_07_partition_lifecycle` | `partitions` / `partition_create_edit_delete_round_trip` |
| `filesystem_lifecycle_runs_in_the_private_storage_lab` | `native_case::case_08_filesystem_lifecycle` | `filesystems` / `filesystem_mount_options_busy_retry_and_cleanup` |
| `image_round_trip_runs_in_the_private_storage_lab` | `native_case::case_09_image_round_trip` | `images` / `image_backup_restore_round_trip_and_readonly_enforcement` |
| `luks_unwind_cleanup_runs_in_the_private_storage_lab` | `native_case::case_10_luks_unwind_cleanup` | `encryption` / `luks_failure_after_format_cleans_auto_opened_mapper` |
| `btrfs_lifecycle_runs_in_the_private_storage_lab` | `native_case::case_11_btrfs_lifecycle` | `btrfs` / `btrfs_subvolume_snapshot_default_and_conflict_round_trip` |
| `local_sftp_runs_in_the_private_storage_lab` | `native_case::case_12_local_sftp` | `network` / `local_sftp_config_test_mount_unmount_and_failures` |
| `lvm_lifecycle_runs_in_the_private_storage_lab` | `native_case::case_13_lvm_lifecycle` | `logical` / `lvm_create_resize_delete_and_stale_review` |
| `mdraid_lifecycle_runs_in_the_private_storage_lab` | `native_case::case_14_mdraid_lifecycle` | `logical` / `mdraid_create_stop_start_delete_and_invalid_members` |
| `application_registry_runs_in_the_private_storage_lab` | `native_case::case_15_application_registry` | `application` / `production_registry_partition_workflow_and_error_mapping` |

### Replaced non-native identities

- `cosmic-ext-storage`: `utils::unit_size_input::tests::test_auto_select` (assertion mapping above).
- `cosmic-ext-storage`: `utils::unit_size_input::tests::test_bytes_to_gigabytes` (assertion mapping above).
- `cosmic-ext-storage`: `utils::unit_size_input::tests::test_bytes_to_megabytes` (assertion mapping above).
- `cosmic-ext-storage`: `utils::unit_size_input::tests::test_gigabytes_to_bytes` (assertion mapping above).
- `cosmic-ext-storage`: `utils::unit_size_input::tests::test_megabytes_to_bytes` (assertion mapping above).
- `cosmic-ext-storage`: `utils::unit_size_input::tests::test_unit_index_roundtrip` (assertion mapping above).
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::keyboard_and_fixture_inputs_cannot_inject_arguments_or_escape_root` (assertion mapping above).
- `ui-e2e-runner::bin/ui-e2e-runner`: `shutdown::tests::changed_scope_expiry_and_supervisor_failure_are_not_quarantined` (assertion mapping above).
- `ui-e2e-runner::bin/ui-e2e-runner`: `shutdown::tests::functional_failure_cannot_be_overridden_by_shutdown_status` (assertion mapping above).
- `ui-e2e-runner::bin/ui-e2e-runner`: `shutdown::tests::quarantine_requires_exact_ordered_stack_after_close` (assertion mapping above).

### Added non-native identities

- `cosmic-ext-storage`: `utils::unit_size_input::tests::bytes_to_unit::case_1_megabytes`.
- `cosmic-ext-storage`: `utils::unit_size_input::tests::bytes_to_unit::case_2_gigabytes`.
- `cosmic-ext-storage`: `utils::unit_size_input::tests::test_auto_select::case_1_bytes`.
- `cosmic-ext-storage`: `utils::unit_size_input::tests::test_auto_select::case_2_kilobytes`.
- `cosmic-ext-storage`: `utils::unit_size_input::tests::test_auto_select::case_3_megabytes`.
- `cosmic-ext-storage`: `utils::unit_size_input::tests::test_auto_select::case_4_gigabytes`.
- `cosmic-ext-storage`: `utils::unit_size_input::tests::test_auto_select::case_5_terabytes`.
- `cosmic-ext-storage`: `utils::unit_size_input::tests::test_unit_index_roundtrip::case_1_bytes`.
- `cosmic-ext-storage`: `utils::unit_size_input::tests::test_unit_index_roundtrip::case_2_kilobytes`.
- `cosmic-ext-storage`: `utils::unit_size_input::tests::test_unit_index_roundtrip::case_3_megabytes`.
- `cosmic-ext-storage`: `utils::unit_size_input::tests::test_unit_index_roundtrip::case_4_gigabytes`.
- `cosmic-ext-storage`: `utils::unit_size_input::tests::test_unit_index_roundtrip::case_5_terabytes`.
- `cosmic-ext-storage`: `utils::unit_size_input::tests::unit_to_bytes::case_1_megabytes`.
- `cosmic-ext-storage`: `utils::unit_size_input::tests::unit_to_bytes::case_2_gigabytes`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::executable_case_rejects_invalid_mutation::case_1_schema`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::executable_case_rejects_invalid_mutation::case_2_timeout`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::executable_case_rejects_invalid_mutation::case_3_action`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::executable_case_rejects_invalid_mutation::case_4_duplicate_step`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::executable_case_rejects_invalid_mutation::case_5_unknown_field`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::fixture_paths_stay_within_root::case_1_local`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::fixture_paths_stay_within_root::case_2_parent`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::fixture_paths_stay_within_root::case_3_absolute`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::fixture_paths_stay_within_root::case_4_symlink`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::keyboard_arguments_are_bounded::case_01_tab`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::keyboard_arguments_are_bounded::case_02_return_key`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::keyboard_arguments_are_bounded::case_03_escape`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::keyboard_arguments_are_bounded::case_04_backspace`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::keyboard_arguments_are_bounded::case_05_space`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::keyboard_arguments_are_bounded::case_06_left`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::keyboard_arguments_are_bounded::case_07_right`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::keyboard_arguments_are_bounded::case_08_up`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::keyboard_arguments_are_bounded::case_09_down`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::keyboard_arguments_are_bounded::case_10_shift_tab`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::keyboard_arguments_are_bounded::case_11_letter`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::keyboard_arguments_are_bounded::case_12_unicode`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::keyboard_arguments_are_bounded::case_13_hyphen`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::keyboard_arguments_are_bounded::case_14_empty`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::keyboard_arguments_are_bounded::case_15_newline`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::keyboard_arguments_are_bounded::case_16_option`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::keyboard_arguments_are_bounded::case_17_shell`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::semantic_properties_require_observed_values::case_1_name`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::semantic_properties_require_observed_values::case_2_description`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::semantic_properties_require_observed_values::case_3_role`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::semantic_properties_require_observed_values::case_4_automation_id`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::semantic_properties_require_observed_values::case_5_unknown_property`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `cases::tests::semantic_properties_require_observed_values::case_6_wrong_value`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `shutdown::tests::functional_and_shutdown_status::case_1_failed_function_clean`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `shutdown::tests::functional_and_shutdown_status::case_2_failed_function_known`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `shutdown::tests::functional_and_shutdown_status::case_3_failed_function_failed`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `shutdown::tests::functional_and_shutdown_status::case_4_failed_function_not_attempted`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `shutdown::tests::functional_and_shutdown_status::case_5_passed_function_failed`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `shutdown::tests::functional_and_shutdown_status::case_6_passed_function_not_attempted`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `shutdown::tests::functional_and_shutdown_status::case_7_passed_function_known`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `shutdown::tests::functional_and_shutdown_status::case_8_passed_function_clean`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `shutdown::tests::policy_evidence_retains_identity`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `shutdown::tests::quarantine_accepts_only_known_stack`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `shutdown::tests::quarantine_rejects_changed_diagnostic::case_1_before_close`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `shutdown::tests::quarantine_rejects_changed_diagnostic::case_2_wrong_signal`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `shutdown::tests::quarantine_rejects_changed_diagnostic::case_3_wrong_kind`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `shutdown::tests::quarantine_rejects_changed_diagnostic::case_4_wrong_schema`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `shutdown::tests::quarantine_rejects_changed_diagnostic::case_5_no_frames`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `shutdown::tests::quarantine_rejects_changed_diagnostic::case_6_reversed_frames`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `shutdown::tests::quarantine_rejects_changed_diagnostic::case_7_wrong_library`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `shutdown::tests::quarantine_rejects_changed_diagnostic::case_8_wrong_owner`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `shutdown::tests::quarantine_rejects_changed_diagnostic::case_9_conflicting_exit`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `shutdown::tests::quarantine_rejects_changed_scope::case_1_missing_supervisor`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `shutdown::tests::quarantine_rejects_changed_scope::case_2_wrong_case`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `shutdown::tests::quarantine_rejects_changed_scope::case_3_wrong_case_hash`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `shutdown::tests::quarantine_rejects_changed_scope::case_4_wrong_environment`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `shutdown::tests::quarantine_rejects_changed_scope::case_5_wrong_lock`.
- `ui-e2e-runner::bin/ui-e2e-runner`: `shutdown::tests::quarantine_rejects_changed_scope::case_6_expired`.
- `test-backend::fixture_composition`: `async_fixture_override::case_1_partition`.
- `test-backend::fixture_composition`: `async_fixture_override::case_2_empty`.
- `test-backend::fixture_composition`: `failure_is_reported_and_resources_unwind::case_1_setup`.
- `test-backend::fixture_composition`: `failure_is_reported_and_resources_unwind::case_2_body`.
- `test-backend::fixture_composition`: `fixture_dependencies_are_factories`.
- `test-backend::fixture_composition`: `independent_mutable_scenarios::case_1_first`.
- `test-backend::fixture_composition`: `independent_mutable_scenarios::case_2_second`.
- `test-backend::fixture_composition`: `owned_fixture_drops_normally`.
- `test-backend::fixture_composition`: `panic_cleanup_probe`.
- `test-backend::fixture_composition`: `setup_failure_probe`.

The new native selection parent/inner probe add exactly one intended container
launch. All other native entry points and safety regressions remain. The
increased count of table cases is not counted as increased production coverage.
