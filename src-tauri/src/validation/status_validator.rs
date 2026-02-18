// Step 2: status state machine

use crate::error::{ErrorCode, ValidationError};
use crate::schema::registry::SchemaRegistry;

/// Result of a status transition validation.
///
/// - `errors`: hard validation failures (e.g., invalid status value). These block the write.
/// - `warnings`: soft constraint notices (e.g., backward transition without reason,
///   skip transitions). These do NOT block the write.
#[derive(Debug, Clone)]
pub struct StatusValidationResult {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
}

impl StatusValidationResult {
    fn ok() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn with_error(error: ValidationError) -> Self {
        Self {
            errors: vec![error],
            warnings: Vec::new(),
        }
    }

    fn with_warnings(warnings: Vec<String>) -> Self {
        Self {
            errors: Vec::new(),
            warnings,
        }
    }
}

/// Validate a status transition for the given entity type.
///
/// Rules:
/// - null -> any valid status: OK (creation, no reason needed)
/// - Same status: OK (idempotent no-op)
/// - Forward transition (earlier index -> later index): OK (no reason needed)
///   - Skip transitions (jumping over intermediate states): OK with a warning
/// - Backward transition (later index -> earlier index): soft constraint (warning, not error)
///   - Without reason: warning suggesting a reason for audit
///   - With reason: informational warning noting the backward transition
/// - Invalid status value: error (hard reject) with valid values listed
pub fn validate_status_transition(
    entity_type: &str,
    current_status: Option<&str>,
    new_status: &str,
    reason: Option<&str>,
) -> StatusValidationResult {
    let valid_statuses = match SchemaRegistry::global().valid_statuses(entity_type) {
        Some(s) => s,
        None => {
            // Unknown entity type -- we can't validate status, but we don't error here
            // since entity type validation is handled elsewhere. Just return empty.
            return StatusValidationResult::ok();
        }
    };

    // Check that new_status is a valid value -- this is still a hard error
    let new_index = match valid_statuses.iter().position(|s| s == new_status) {
        Some(idx) => idx,
        None => {
            let joined = valid_statuses.join(", ");
            return StatusValidationResult::with_error(ValidationError {
                code: ErrorCode::InvalidStatusTransition,
                field_path: "status".to_string(),
                message: format!(
                    "Invalid status '{}' for entity type '{}'. Valid statuses: [{}]",
                    new_status, entity_type, joined
                ),
                expected: Some(format!("one of [{}]", joined)),
                actual: Some(new_status.to_string()),
            });
        }
    };

    // null -> any valid status is always OK
    let current = match current_status {
        None => return StatusValidationResult::ok(),
        Some(c) => c,
    };

    // Same status is idempotent, always OK
    if current == new_status {
        return StatusValidationResult::ok();
    }

    // Find current status index
    let current_index = match valid_statuses.iter().position(|s| s == current) {
        Some(idx) => idx,
        None => {
            // Current status is invalid (data corruption?). Allow the transition
            // to a valid status without requiring reason, since we can't determine direction.
            return StatusValidationResult::ok();
        }
    };

    // Forward transition: OK, but check for skip
    if new_index > current_index {
        let mut warnings = Vec::new();
        if new_index - current_index > 1 {
            // Skip transition: jumping over intermediate states
            let skipped: Vec<&str> = valid_statuses[current_index + 1..new_index]
                .iter()
                .map(|s| s.as_str())
                .collect();
            warnings.push(format!(
                "Status skip transition from '{}' to '{}' (skipped: {})",
                current,
                new_status,
                skipped.join(", ")
            ));
        }
        return StatusValidationResult::with_warnings(warnings);
    }

    // Backward transition: soft constraint (warning, not error)
    if new_index < current_index {
        let mut warnings = Vec::new();
        match reason {
            Some(r) if !r.trim().is_empty() => {
                // Reason provided -- informational warning noting the backward transition
                warnings.push(format!(
                    "Backward status transition from '{}' to '{}' with reason: {}",
                    current, new_status, r.trim()
                ));
            }
            _ => {
                // No reason provided -- warning suggesting a reason for audit
                warnings.push(format!(
                    "Backward status transition from '{}' to '{}' without reason. Consider providing a reason for audit.",
                    current, new_status
                ));
            }
        }
        return StatusValidationResult::with_warnings(warnings);
    }

    StatusValidationResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Metric status tests ---

    #[test]
    fn test_metric_null_to_active() {
        let result = validate_status_transition("metric", None, "active", None);
        assert!(result.errors.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_metric_null_to_archived() {
        let result = validate_status_transition("metric", None, "archived", None);
        assert!(result.errors.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_metric_forward_active_to_paused() {
        let result = validate_status_transition("metric", Some("active"), "paused", None);
        assert!(result.errors.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_metric_forward_active_to_archived() {
        let result = validate_status_transition("metric", Some("active"), "archived", None);
        assert!(result.errors.is_empty());
        // Skip warning: skips paused and deprecated
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("skip"));
        assert!(result.warnings[0].contains("paused"));
        assert!(result.warnings[0].contains("deprecated"));
    }

    #[test]
    fn test_metric_same_status_idempotent() {
        let result = validate_status_transition("metric", Some("active"), "active", None);
        assert!(result.errors.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_metric_backward_without_reason() {
        let result = validate_status_transition("metric", Some("paused"), "active", None);
        // No errors -- backward transitions are soft constraints
        assert!(result.errors.is_empty());
        // But there is a warning
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("Backward status transition"));
        assert!(result.warnings[0].contains("without reason"));
        assert!(result.warnings[0].contains("paused"));
        assert!(result.warnings[0].contains("active"));
    }

    #[test]
    fn test_metric_backward_with_reason() {
        let result = validate_status_transition("metric", Some("paused"), "active", Some("reactivating"));
        assert!(result.errors.is_empty());
        // Informational warning noting the backward transition with reason
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("Backward status transition"));
        assert!(result.warnings[0].contains("reactivating"));
    }

    #[test]
    fn test_metric_backward_empty_reason_warns() {
        let result = validate_status_transition("metric", Some("paused"), "active", Some("  "));
        // No error -- but a warning about missing reason
        assert!(result.errors.is_empty());
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("without reason"));
    }

    #[test]
    fn test_metric_invalid_status() {
        let result = validate_status_transition("metric", Some("active"), "invalid_status", None);
        assert_eq!(result.errors.len(), 1);
        assert!(matches!(result.errors[0].code, ErrorCode::InvalidStatusTransition));
        assert!(result.errors[0].message.contains("Invalid status"));
    }

    // --- Experiment status tests ---

    #[test]
    fn test_experiment_forward_draft_to_running() {
        let result = validate_status_transition("experiment", Some("draft"), "running", None);
        assert!(result.errors.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_experiment_backward_running_to_draft_warns() {
        let result = validate_status_transition("experiment", Some("running"), "draft", None);
        // No errors -- backward is a soft constraint now
        assert!(result.errors.is_empty());
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("Backward"));
    }

    #[test]
    fn test_experiment_backward_with_reason() {
        let result = validate_status_transition("experiment", Some("running"), "draft", Some("needs redesign"));
        assert!(result.errors.is_empty());
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("needs redesign"));
    }

    // --- Result status tests ---

    #[test]
    fn test_result_forward_preliminary_to_final() {
        let result = validate_status_transition("result", Some("preliminary"), "final", None);
        assert!(result.errors.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_result_backward_final_to_preliminary_warns() {
        let result = validate_status_transition("result", Some("final"), "preliminary", None);
        assert!(result.errors.is_empty());
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("Backward"));
    }

    #[test]
    fn test_result_backward_with_reason() {
        let result = validate_status_transition("result", Some("final"), "preliminary", Some("corrections needed"));
        assert!(result.errors.is_empty());
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("corrections needed"));
    }

    // --- Unknown entity type ---

    #[test]
    fn test_unknown_entity_type_no_errors() {
        let result = validate_status_transition("unknown_type", Some("a"), "b", None);
        assert!(result.errors.is_empty());
        assert!(result.warnings.is_empty());
    }

    // --- Edge case: current status invalid in schema ---

    #[test]
    fn test_corrupt_current_status_allows_transition() {
        let result = validate_status_transition("metric", Some("bogus"), "active", None);
        assert!(result.errors.is_empty());
        assert!(result.warnings.is_empty());
    }

    // --- Skip transition tests ---

    #[test]
    fn test_experiment_skip_draft_to_archived() {
        let result = validate_status_transition("experiment", Some("draft"), "archived", None);
        assert!(result.errors.is_empty());
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("skip"));
        assert!(result.warnings[0].contains("running"));
        assert!(result.warnings[0].contains("concluded"));
    }

    #[test]
    fn test_task_skip_open_to_done() {
        let result = validate_status_transition("task", Some("open"), "done", None);
        assert!(result.errors.is_empty());
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("skip"));
        assert!(result.warnings[0].contains("in_progress"));
        assert!(result.warnings[0].contains("blocked"));
    }

    #[test]
    fn test_forward_single_step_no_skip_warning() {
        // Adjacent forward transitions should NOT produce a skip warning
        let result = validate_status_transition("experiment", Some("draft"), "running", None);
        assert!(result.errors.is_empty());
        assert!(result.warnings.is_empty());
    }
}
