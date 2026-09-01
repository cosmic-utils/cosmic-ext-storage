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

## Commands

Minimum commands after implementation:

~~~text
just ui-plan-check
python3 tools/ui-testing/assert_tests.py --plan-only
just ui-assert-tests phase=all
cargo test -p storage-contracts --locked --test ui_workflow_contract
cargo test -p test-backend --locked
cargo test -p cosmic-ext-storage --locked --features test-backend --test ui_runtime_contract
cargo test -p cosmic-ext-storage --locked --test scenario_feature_disabled_contract
cargo test -p cosmic-ext-storage --locked --features test-backend --test ui_scenario_contract
cargo test -p cosmic-ext-storage --locked --features test-backend --test application_workflows
cargo test --workspace --all-features --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
just ui-scenario-check
just harness-nondestructive
just ui-e2e
just package-check
~~~

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

The E2E capability job starts with a private user runtime directory and D-Bus
session, must not connect to the system bus, and builds the binary inside the
locked image with `test-backend` before launching it using `--backend scenario`.
It records the Containerfile base-image digest, package/font lock, Sway command,
capture/input helpers, viewport, and image-comparison settings in its evidence.
A successful capability run proves no real device is needed and that the
compositor prerequisites work; it does not yet substitute for the per-case
semantic and golden comparisons, `harness-nondestructive`, or a disposable
full-lab run.

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
