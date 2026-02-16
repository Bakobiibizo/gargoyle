// Step 2: status state machine

use crate::error::{ErrorCode, ValidationError};

/// Valid status progressions per entity type.
/// The order defines forward (left-to-right) vs backward (right-to-left) transitions.
const METRIC_STATUSES: &[&str] = &["active", "paused", "deprecated", "archived"];
const EXPERIMENT_STATUSES: &[&str] = &["draft", "running", "concluded", "archived"];
const RESULT_STATUSES: &[&str] = &["draft", "final", "archived"];
const TASK_STATUSES: &[&str] = &["backlog", "todo", "in_progress", "blocked", "done", "archived"];
const PROJECT_STATUSES: &[&str] = &["planning", "active", "paused", "completed", "archived"];
const DECISION_STATUSES: &[&str] = &["proposed", "accepted", "deprecated", "superseded"];
const PERSON_STATUSES: &[&str] = &["active", "inactive", "archived"];
const NOTE_STATUSES: &[&str] = &["draft", "final", "archived"];
const SESSION_STATUSES: &[&str] = &["scheduled", "in_progress", "completed", "cancelled"];
const CAMPAIGN_STATUSES: &[&str] = &["planning", "active", "paused", "completed", "archived"];
const AUDIENCE_STATUSES: &[&str] = &["draft", "validated", "active", "archived"];
const COMPETITOR_STATUSES: &[&str] = &["tracking", "dormant", "archived"];
const CHANNEL_STATUSES: &[&str] = &["evaluating", "active", "scaling", "paused", "deprecated"];
const SPEC_STATUSES: &[&str] = &["draft", "review", "approved", "deprecated"];
const BUDGET_STATUSES: &[&str] = &["draft", "approved", "active", "closed"];
const VENDOR_STATUSES: &[&str] = &["evaluating", "active", "on_hold", "terminated"];
const PLAYBOOK_STATUSES: &[&str] = &["draft", "active", "deprecated", "archived"];
const TAXONOMY_STATUSES: &[&str] = &["draft", "active", "archived"];
const BACKLOG_STATUSES: &[&str] = &["open", "triaged", "scheduled", "closed"];
const BRIEF_STATUSES: &[&str] = &["draft", "review", "approved", "archived"];
const EVENT_STATUSES: &[&str] = &["proposed", "confirmed", "in_progress", "completed", "cancelled"];
const POLICY_STATUSES: &[&str] = &["draft", "active", "under_review", "deprecated"];

/// Get the valid status list for a given entity type.
fn statuses_for_entity_type(entity_type: &str) -> Option<&'static [&'static str]> {
    match entity_type {
        "metric" => Some(METRIC_STATUSES),
        "experiment" => Some(EXPERIMENT_STATUSES),
        "result" => Some(RESULT_STATUSES),
        "task" => Some(TASK_STATUSES),
        "project" => Some(PROJECT_STATUSES),
        "decision" => Some(DECISION_STATUSES),
        "person" => Some(PERSON_STATUSES),
        "note" => Some(NOTE_STATUSES),
        "session" => Some(SESSION_STATUSES),
        "campaign" => Some(CAMPAIGN_STATUSES),
        "audience" => Some(AUDIENCE_STATUSES),
        "competitor" => Some(COMPETITOR_STATUSES),
        "channel" => Some(CHANNEL_STATUSES),
        "spec" => Some(SPEC_STATUSES),
        "budget" => Some(BUDGET_STATUSES),
        "vendor" => Some(VENDOR_STATUSES),
        "playbook" => Some(PLAYBOOK_STATUSES),
        "taxonomy" => Some(TAXONOMY_STATUSES),
        "backlog" => Some(BACKLOG_STATUSES),
        "brief" => Some(BRIEF_STATUSES),
        "event" => Some(EVENT_STATUSES),
        "policy" => Some(POLICY_STATUSES),
        _ => None,
    }
}

/// Validate a status transition for the given entity type.
///
/// Rules:
/// - null -> any valid status: OK (creation, no reason needed)
/// - Same status: OK (idempotent no-op)
/// - Forward transition (earlier index -> later index): OK (no reason needed)
/// - Backward transition (later index -> earlier index): requires `reason`, error if missing
/// - Invalid status value: error with valid values listed
pub fn validate_status_transition(
    entity_type: &str,
    current_status: Option<&str>,
    new_status: &str,
    reason: Option<&str>,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    let valid_statuses = match statuses_for_entity_type(entity_type) {
        Some(s) => s,
        None => {
            // Unknown entity type -- we can't validate status, but we don't error here
            // since entity type validation is handled elsewhere. Just return empty.
            return errors;
        }
    };

    // Check that new_status is a valid value
    let new_index = match valid_statuses.iter().position(|&s| s == new_status) {
        Some(idx) => idx,
        None => {
            errors.push(ValidationError {
                code: ErrorCode::InvalidStatusTransition,
                field_path: "status".to_string(),
                message: format!(
                    "Invalid status '{}' for entity type '{}'. Valid statuses: [{}]",
                    new_status,
                    entity_type,
                    valid_statuses.join(", ")
                ),
                expected: Some(format!("one of [{}]", valid_statuses.join(", "))),
                actual: Some(new_status.to_string()),
            });
            return errors;
        }
    };

    // null -> any valid status is always OK
    let current = match current_status {
        None => return errors,
        Some(c) => c,
    };

    // Same status is idempotent, always OK
    if current == new_status {
        return errors;
    }

    // Find current status index
    let current_index = match valid_statuses.iter().position(|&s| s == current) {
        Some(idx) => idx,
        None => {
            // Current status is invalid (data corruption?). Allow the transition
            // to a valid status without requiring reason, since we can't determine direction.
            return errors;
        }
    };

    // Forward transition: OK
    if new_index > current_index {
        return errors;
    }

    // Backward transition: requires reason
    if new_index < current_index {
        match reason {
            Some(r) if !r.trim().is_empty() => {
                // Reason provided, backward transition is allowed
            }
            _ => {
                errors.push(ValidationError {
                    code: ErrorCode::InvalidStatusTransition,
                    field_path: "status".to_string(),
                    message: format!(
                        "Backward status transition from '{}' to '{}' requires a reason",
                        current, new_status
                    ),
                    expected: Some("reason field must be provided for backward transitions".to_string()),
                    actual: None,
                });
            }
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Metric status tests ---

    #[test]
    fn test_metric_null_to_active() {
        let errors = validate_status_transition("metric", None, "active", None);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_metric_null_to_archived() {
        let errors = validate_status_transition("metric", None, "archived", None);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_metric_forward_active_to_paused() {
        let errors = validate_status_transition("metric", Some("active"), "paused", None);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_metric_forward_active_to_archived() {
        let errors = validate_status_transition("metric", Some("active"), "archived", None);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_metric_same_status_idempotent() {
        let errors = validate_status_transition("metric", Some("active"), "active", None);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_metric_backward_without_reason() {
        let errors = validate_status_transition("metric", Some("paused"), "active", None);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::InvalidStatusTransition));
        assert!(errors[0].message.contains("requires a reason"));
    }

    #[test]
    fn test_metric_backward_with_reason() {
        let errors = validate_status_transition("metric", Some("paused"), "active", Some("reactivating"));
        assert!(errors.is_empty());
    }

    #[test]
    fn test_metric_backward_empty_reason_rejected() {
        let errors = validate_status_transition("metric", Some("paused"), "active", Some("  "));
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::InvalidStatusTransition));
    }

    #[test]
    fn test_metric_invalid_status() {
        let errors = validate_status_transition("metric", Some("active"), "invalid_status", None);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::InvalidStatusTransition));
        assert!(errors[0].message.contains("Invalid status"));
    }

    // --- Experiment status tests ---

    #[test]
    fn test_experiment_forward_draft_to_running() {
        let errors = validate_status_transition("experiment", Some("draft"), "running", None);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_experiment_backward_running_to_draft_needs_reason() {
        let errors = validate_status_transition("experiment", Some("running"), "draft", None);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn test_experiment_backward_with_reason() {
        let errors = validate_status_transition("experiment", Some("running"), "draft", Some("needs redesign"));
        assert!(errors.is_empty());
    }

    // --- Result status tests ---

    #[test]
    fn test_result_forward_draft_to_final() {
        let errors = validate_status_transition("result", Some("draft"), "final", None);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_result_backward_final_to_draft_needs_reason() {
        let errors = validate_status_transition("result", Some("final"), "draft", None);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn test_result_backward_with_reason() {
        let errors = validate_status_transition("result", Some("final"), "draft", Some("corrections needed"));
        assert!(errors.is_empty());
    }

    // --- Unknown entity type ---

    #[test]
    fn test_unknown_entity_type_no_errors() {
        let errors = validate_status_transition("unknown_type", Some("a"), "b", None);
        assert!(errors.is_empty());
    }

    // --- Edge case: current status invalid in schema ---

    #[test]
    fn test_corrupt_current_status_allows_transition() {
        let errors = validate_status_transition("metric", Some("bogus"), "active", None);
        assert!(errors.is_empty());
    }
}
