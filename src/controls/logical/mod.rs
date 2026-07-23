//! Shared logical-storage control text and capability presentation.

use storage_types::{LogicalCapabilities, LogicalOperation};

pub(crate) fn operation_label(operation: LogicalOperation) -> &'static str {
    match operation {
        LogicalOperation::Create => "Create",
        LogicalOperation::Delete => "Delete",
        LogicalOperation::Resize => "Resize",
        LogicalOperation::AddMember => "Add member",
        LogicalOperation::RemoveMember => "Remove member",
        LogicalOperation::Activate => "Activate",
        LogicalOperation::Deactivate => "Deactivate",
        LogicalOperation::Start => "Start",
        LogicalOperation::Stop => "Stop",
        LogicalOperation::Check => "Check",
        LogicalOperation::Repair => "Repair",
        LogicalOperation::SetLabel => "Set label",
        LogicalOperation::SetDefaultSubvolume => "Set default subvolume",
    }
}

pub(crate) fn operation_status(
    capabilities: &LogicalCapabilities,
    operation: LogicalOperation,
) -> Result<(), String> {
    if capabilities.is_allowed(operation) {
        Ok(())
    } else {
        Err(capabilities
            .blocked_reason(operation)
            .unwrap_or("Unavailable")
            .to_string())
    }
}
