// Step 3: expected_updated_at check (optimistic locking)

use crate::error::{ErrorCode, ValidationError};

/// Validate optimistic locking by comparing the expected_updated_at timestamp
/// from the patch operation against the actual updated_at value from the database.
///
/// Uses exact string comparison. If they don't match, returns a LockConflict error
/// indicating that another write has occurred since the client last read the entity.
pub fn validate_lock(expected: &str, actual: &str) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if expected != actual {
        errors.push(ValidationError {
            code: ErrorCode::LockConflict,
            field_path: "expected_updated_at".to_string(),
            message: format!(
                "Optimistic lock conflict: expected updated_at '{}' but found '{}'. \
                 The entity has been modified since you last read it.",
                expected, actual
            ),
            expected: Some(expected.to_string()),
            actual: Some(actual.to_string()),
        });
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matching_timestamps_no_error() {
        let errors = validate_lock("2025-01-15T10:30:00Z", "2025-01-15T10:30:00Z");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_mismatched_timestamps_lock_conflict() {
        let errors = validate_lock("2025-01-15T10:30:00Z", "2025-01-15T11:00:00Z");
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::LockConflict));
        assert_eq!(errors[0].expected.as_deref(), Some("2025-01-15T10:30:00Z"));
        assert_eq!(errors[0].actual.as_deref(), Some("2025-01-15T11:00:00Z"));
    }

    #[test]
    fn test_exact_string_comparison() {
        // Same instant but different string representations should conflict
        let errors = validate_lock("2025-01-15T10:30:00Z", "2025-01-15T10:30:00.000Z");
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].code, ErrorCode::LockConflict));
    }

    #[test]
    fn test_empty_strings_match() {
        let errors = validate_lock("", "");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_whitespace_difference_conflicts() {
        let errors = validate_lock("2025-01-15T10:30:00Z", "2025-01-15T10:30:00Z ");
        assert_eq!(errors.len(), 1);
    }
}
