# Validation

## Required automated coverage

The names below are required, not examples. They are stored once in
`tests/ui/required-tests.toml`; [traceability.md](traceability.md) links each
to a method/UI branch and `just ui-assert-tests phase=all` lists every named
test from its real target before running it. A renamed, filtered-out, ignored,
duplicated, or wrong-target test fails visibly. This document and the manifest
must match exactly.

| Area | Required tests |
| --- | --- |
| Portable runtime contracts | `workflow_contracts_are_object_safe`; `workflow_values_are_serde_stable`; `desktop_services_are_not_storage_backend_methods`; `operation_id_has_deterministic_scenario_constructor`; `runtime_adapters_reject_invalid_registration_order` |
| Schema | `rejects_unknown_or_unsupported_schema`; `rejects_schema_version_that_is_not_first_lexical_key`; `rejects_ambiguous_behaviour_rules`; `rejects_invalid_cross_references`; `scenario_round_trip_is_deterministic`; `overlay_write_is_atomic_and_never_rewrites_fixture`; `query_snapshot_and_subscription_cutover_are_linearized`; `scenario_control_is_actor_serialized` |
| Safety and factory | `scenario_factory_never_constructs_production_adapters`; `scenario_mode_has_no_real_backend_fallback`; `feature_disabled_scenario_request_fails_before_real_factory`; `scenario_crate_has_no_production_adapter_dependencies`; `scenario_store_is_the_only_host_io_boundary`; `trace_redacts_passphrases_and_descriptors` |
| Trait completeness | `scenario_operation_inventory_matches_contract_surface`; `bootstrap_inventory_matches_generated_contract_surface`; `unsupported_methods_never_succeed_or_touch_host`; `native_only_legacy_image_methods_return_unsupported`; `generated_cases_cover_success_error_trace_and_transition_for_every_modelled_operation`; one success, configured-error, trace, and state-transition case for every block/Btrfs/logical/network/usage/image/tool method |
| Mutation/events | `partition_transition_emits_ordered_device_event`; `busy_unmount_preserves_state`; `logical_confirmation_requires_current_preflight_key`; `logical_mutation_emits_one_declared_refresh_event`; `btrfs_utility_state_never_uses_host_tools`; `network_mutations_follow_declared_schema`; `scenario_reload_is_atomic_and_refreshes_once`; `cancelled_image_operation_cannot_complete_successfully`; `legacy_image_device_contract_is_removed_atomically` |
| Application composition | `app_runtime_uses_injected_operations_for_startup_and_updates`; `device_subscription_uses_selected_runtime`; `background_tasks_do_not_reconstruct_operations`; `production_runtime_constructs_real_registry_once`; `runtime_test_facade_dispatches_without_desktop_server` |
| Production workflow migration | `usage_workflow_is_routed_through_contract`; `image_workflow_is_routed_through_contract`; `desktop_actions_are_routed_through_services`; `production_workflow_semantics_are_preserved` |
| In-process UI | `physical_scenario_drives_dialog_and_refresh`; `configured_backend_error_remains_actionable`; `network_scenario_exercises_crud_and_mount_state`; `usage_and_image_progress_render_from_contract_events`; `keyboard_dialog_flow_has_named_controls` |
| Application workflow integration | `workflow_harness_uses_only_selected_scenario_runtime`; `logical_open_preflight_confirm_executes_once_and_refreshes_once`; `logical_stale_preflight_completion_is_rejected`; `partition_format_validation_and_completion_preserve_effect_order`; `busy_unmount_keeps_actionable_error_and_does_not_refresh`; `luks_unlock_uses_secret_input_and_redacts_every_projection`; `network_create_mount_and_status_are_reduced_from_one_flow`; `image_progress_cancel_and_terminal_state_are_virtual_clock_driven`; `image_usage_stale_completion_cannot_replace_newer_workflow_state`; `usage_scan_and_delete_map_results_without_host_file_access`; `scenario_reload_is_atomic_and_generation_checked_by_the_application`; `reload_stale_completion_cannot_move_virtual_time_backwards`; `workflow_effects_do_not_call_global_operations_context`; `workflow_facade_covers_every_migrated_path` |
| Runner/environment | `png_signature_requires_all_eight_bytes`; `legacy_case_inventory_is_sorted_and_rejects_duplicate_ids`; `ui-e2e` capability gate |
| E2E a11y/visual | `physical_partition_format`; `busy_unmount`; `luks_unlock`; `logical_preflight_confirmation`; `network_mount`; `image_usage_progress`; `keyboard_accessibility`; `live_scenario_reload` |

The trait-completeness inventory is deliberate. A contract change is not
complete until its scenario entry, schema validation, behaviour matcher, trace
redaction, default/error tests, and any needed fixture have been added.

The inventory is generated from the `storage-contracts` contract-surface
manifest, not maintained as a second string list. `native_only` is legal only
for a method explicitly marked there; it must return a typed `Unsupported`
error and cannot touch scenario state, events, or host I/O.

`feature_disabled_scenario_request_fails_before_real_factory` is intentionally
compiled in the separate no-feature `scenario_feature_disabled_contract`
target. It builds and launches a second no-feature binary artifact, asserts its
pre-window stderr/exit contract and a zero production-factory probe, and does
not infer compile-time feature absence from a `test-backend` test binary.
`same_tick_cancel_order_is_deterministic` runs in Phase 6, after the image/usage
operation model exists; Phase 5 covers only serialized clock control.

## rstest composition regression gate (Testing V2 addition)

The `rstest` phase adds the `test-backend` integration target
`fixture_composition`. Its maintained mandatory identities are:

- `async_fixture_override::case_1_partition`
- `async_fixture_override::case_2_empty`
- `independent_mutable_scenarios::case_1_first`
- `independent_mutable_scenarios::case_2_second`
- `owned_fixture_drops_normally`
- `fixture_dependencies_are_factories`
- `failure_is_reported_and_resources_unwind::case_1_setup`
- `failure_is_reported_and_resources_unwind::case_2_body`

The two failure parents execute their ignored child probes and verify nonzero
exit and owned-resource cleanup. The probes are not independent passing gates.
These composition regressions supplement, rather than replace, the semantic
workflow and executed UI obligations. See the
[adoption plan](../5-testing-v2/rstest-implementation-plan.md).

## Commands

Minimum commands after implementation:

> Historical command/diagram block retired by [Testing V2](../5-testing-v2/spec.md); [original record](../5-testing-v2/legacy-harness-history.md#h041).

`just ui-scenario-check` validates every
`tests/ui/scenarios/**/*.toml`, checks the frozen schema descriptor and
traceability matrix, runs the scenario crate tests, and fails if any checked-in
scenario is unused by both a
contract test and a named UI case unless it is marked as a documented manual
fixture. `just ui-e2e` runs in the same pinned container/session model as CI.

## CI acceptance

`ui-scenario-contract` and `ui-e2e` are required PR checks. On success and
failure, `ui-e2e` uploads one `ui-artifacts/` directory containing:

- case manifest and copied scenario/overlay;
- scenario trace and application log;
- AT-SPI accessibility tree/action log;
- actual screenshot, reviewed expected screenshot, and pixel diff for each
  checkpoint;
- JUnit result and a human-readable per-case summary.

> Historical testing-infrastructure note superseded by [Testing V2](../5-testing-v2/spec.md); [original record](../5-testing-v2/legacy-harness-history.md#h042).

## Manual test trace

On a regular COSMIC Wayland session, build with `test-backend` and launch a
copy of a scenario using a writable overlay. Confirm the clearly visible test
scenario indicator before beginning.

1. Edit disk, partition, filesystem, logical, and network values in the
   overlay; validate it with `ui-scenario validate`. Confirm a valid watched
   reload refreshes the matching screen and an invalid edit leaves it intact.
2. Perform a successful partition/format/mount flow and confirm only overlay
   and trace change, never a host disk or mount.
3. Exercise configured permission, busy, unavailable, and delayed-progress
   errors. Confirm controls recover, text is actionable, and a stale completion
   never overwrites later state.
4. Perform Btrfs/LVM/MD logical preflight and confirmation flows. Confirm stale
   preflight keys conflict and device-event refresh occurs once.
5. Use keyboard-only navigation through every test-case dialog. Confirm every
   icon/action, form field, disabled reason, progress indicator, and dialog
   button has an accessible name and useful state.
6. Trigger image backup/restore/cancel and usage scan/delete. Confirm no path
   outside the scenario overlay is accessed and trace redacts sensitive inputs.
7. Rebuild and launch without `test-backend`. Confirm normal launch contains no
   scenario indicator or controls and continues to use real UDisks.

Record screenshots and the scenario trace with the validation result. Run the
existing disposable-fixture harness separately when validating changes that
also affect real-adapter behaviour.

## Testing V2 remaining-work regression additions (2026-09-14)

`cosmic-ext-storage --features test-backend --test volume_models` uses the
owned rstest scenario fixture without requiring host D-Bus. Its first two
tests retain the old simple/nested tree assertions, replacing silent early
returns when a system bus was unavailable. All names below are mandatory:

- `test_build_simple_tree`
- `test_build_nested_tree`
- `tree_filters_roots::case_1_empty`
- `tree_filters_roots::case_2_other_disk`
- `tree_filters_roots::case_3_orphan`
- `tree_filters_roots::case_4_no_device`
- `nested_mutation_search_mount_and_clone_preserve_ownership`

The target is explicitly scenario-feature-gated; it does not silently pass
without running assertions. Native resource ownership and all destructive
tests remain in the private storage lab.

### Active create-wizard validation regressions (2026-09-14)

Required root library tests use Cargo's standard `--lib -- --list`, with no
new test dispatcher. These rstest fixtures own plain dialog values; no host
storage or global runtime is used.
The validator also checks the separate `--ignored --list` output for every
Rust target: ordinary listing alone cannot distinguish ignored required tests.

- `update::volumes::create::tests::basics_requires_a_known_available_filesystem::case_1_available`
- `update::volumes::create::tests::basics_requires_a_known_available_filesystem::case_2_unavailable`
- `update::volumes::create::tests::basics_requires_a_known_available_filesystem::case_3_unknown_table`
- `update::volumes::create::tests::basics_requires_a_known_available_filesystem::case_4_stale_index`
- `update::volumes::create::tests::sizing_requires_a_nonzero_size_within_the_extent::case_1_zero`
- `update::volumes::create::tests::sizing_requires_a_nonzero_size_within_the_extent::case_2_minimum`
- `update::volumes::create::tests::sizing_requires_a_nonzero_size_within_the_extent::case_3_maximum`
- `update::volumes::create::tests::sizing_requires_a_nonzero_size_within_the_extent::case_4_oversized`
- `update::volumes::create::tests::sizing_requires_a_nonzero_size_within_the_extent::case_5_overflow_boundary`
- `update::volumes::create::tests::final_step_does_not_reapply_basics_or_sizing_navigation_checks`

### Active UI runtime-directory regressions (2026-09-14)

- `long_artifact_paths_have_short_owned_sockets_and_preserve_evidence`
- `existing_artifact_directory_is_never_removed`
- `keyboard_keeper_is_reaped_and_early_exit_is_rejected`

These owned rstest cases bind real Unix sockets under a short private temporary
runtime directory, even with long artifact paths. They verify runtime/token
cleanup while preserving evidence and rejecting existing artifact directories.
Keyboard cases retain an owned virtual keyboard device for the case lifetime;
its regression rejects early process exit and verifies child termination/reaping.
# Non-rendered production-handler migration (2026-09-16)

Required mount/unmount production-route cases:
- `update::production_handler_tests::mount::busy_result_routes_to_dialog_and_retry_preserves_mounted_state::case_1_segment`
- `update::production_handler_tests::mount::busy_result_routes_to_dialog_and_retry_preserves_mounted_state::case_2_child`
- `update::production_handler_tests::mount::busy_result_routes_to_dialog_and_retry_preserves_mounted_state::case_3_sidebar`
- `update::production_handler_tests::mount::unsupported_kill_is_an_error_not_a_fake_unmount_success`
- `update::production_handler_tests::mount::child_mount_and_unmount_refresh_real_state`

Required encryption production-route cases:
- `update::production_handler_tests::encryption::wrong_secret_retries_successfully_then_locks_actual_model`
- `update::production_handler_tests::encryption::cancelled_or_missing_target_never_unlocks::case_1_cancel`
- `update::production_handler_tests::encryption::cancelled_or_missing_target_never_unlocks::case_2_missing_partition`
- `update::production_handler_tests::encryption::cancelled_unlock_failure_cannot_reopen_a_secret_dialog`

Required logical production-route cases (replace the parallel logical workflow):
- `update::production_handler_tests::logical::confirmation_executes_once_and_schedules_both_real_refreshes`
- `update::production_handler_tests::logical::cancelled_confirmation_and_changed_identity_cannot_execute`
- `update::production_handler_tests::logical::cancelled_preflight_cannot_reopen_confirmation`
- `update::production_handler_tests::logical::stale_candidate_capture_cannot_change_current_selection::case_1_replaced`
- `update::production_handler_tests::logical::stale_candidate_capture_cannot_change_current_selection::case_2_left_view`
- `update::production_handler_tests::logical::stale_topology_does_not_replace_current_candidate_resolution`
- `update::production_handler_tests::logical::operation_failure_clears_pending_without_success_refresh`
- `update::production_handler_tests::logical::action_form_validates_input_and_cancel_clears_actual_dialog`

Required stale/duplicate completion cases:
- `update::production_handler_tests::old_partition_completion_cannot_replace_a_new_running_dialog::case_1_success`
- `update::production_handler_tests::old_partition_completion_cannot_replace_a_new_running_dialog::case_2_failure`

The parallel partition workflow assertion is superseded by these required
production-handler tests. They execute real messages and typed task outputs
against the selected scenario runtime without rendering:

- `update::production_handler_tests::create_routes_to_selected_runtime_and_refreshes_actual_models`
- `update::production_handler_tests::create_cancel_and_missing_dialog_have_no_effect::case_1_cancel`
- `update::production_handler_tests::create_cancel_and_missing_dialog_have_no_effect::case_2_no_dialog`
- `update::production_handler_tests::create_submission_revalidates_form_without_side_effects::case_1_zero_size`
- `update::production_handler_tests::create_submission_revalidates_form_without_side_effects::case_2_oversized`
- `update::production_handler_tests::create_submission_revalidates_form_without_side_effects::case_3_missing_tool`
- `update::production_handler_tests::create_submission_revalidates_form_without_side_effects::case_4_missing_secret`
- `update::production_handler_tests::create_submission_revalidates_form_without_side_effects::case_5_mismatched_secret`
- `update::production_handler_tests::create_failure_surfaces_error_without_refreshing_navigation`
- `update::production_handler_tests::format_uses_selected_runtime_and_handles_invalid_targets::case_1_success`
- `update::production_handler_tests::format_uses_selected_runtime_and_handles_invalid_targets::case_2_missing_device`
- `update::production_handler_tests::format_uses_selected_runtime_and_handles_invalid_targets::case_3_missing_tool`
- `update::production_handler_tests::create_completion_does_not_close_a_replacement_idle_dialog`
# Production network handler migration (2026-09-16)

The parallel network workflow is replaced by these real message-handler tests:

- `update::production_handler_tests::network::create_select_mount_status_unmount_use_real_handlers`
- `update::production_handler_tests::network::create_validation_conflict_and_retry_preserve_form`
- `update::production_handler_tests::network::cancelled_create_completion_cannot_replace_new_wizard`
- `update::production_handler_tests::network::editor_save_rejects_duplicate_and_stale_completion`
- `update::production_handler_tests::network::old_status_cannot_overwrite_mount_and_errors_are_visible`
- `update::production_handler_tests::network::invalid_provider_or_missing_schema_field_never_submits::case_1_unsupported`
- `update::production_handler_tests::network::invalid_provider_or_missing_schema_field_never_submits::case_2_missing_required`
- `update::production_handler_tests::network::editor_conflict_retry_updates_backend_and_completion_is_once`
# Production image/usage migration (2026-09-16)

- `update::production_handler_tests::image_usage::image_start_uses_selected_adapter_and_rejects_duplicate`
- `update::production_handler_tests::image_usage::stale_image_start_cannot_bind_to_replacement_dialog`
- `update::production_handler_tests::image_usage::image_progress_terminal_and_cleanup_use_actual_subscription_messages::case_1_complete`
- `update::production_handler_tests::image_usage::image_progress_terminal_and_cleanup_use_actual_subscription_messages::case_2_cancel`
- `update::production_handler_tests::image_usage::image_cancel_before_start_completion_is_honoured`
- `update::production_handler_tests::image_usage::image_validation_and_adapter_errors_allow_retry::case_1_missing_path`
- `update::production_handler_tests::image_usage::image_validation_and_adapter_errors_allow_retry::case_2_unknown_asset`
- `update::production_handler_tests::image_usage::image_validation_and_adapter_errors_allow_retry::case_3_host_path`
- `update::production_handler_tests::image_usage::usage_wizard_scan_selection_delete_and_refresh_use_real_state`
- `update::production_handler_tests::image_usage::usage_old_scan_and_delete_cannot_change_replacement_state`
- `update::production_handler_tests::image_usage::usage_cancelled_wizard_and_invalid_selection_never_start`
- `update::production_handler_tests::image_usage::usage_running_adapter_waits_for_virtual_completion`
- `update::production_handler_tests::image_usage::usage_unsupported_adapter_surfaces_error_without_success`
