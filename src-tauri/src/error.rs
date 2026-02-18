use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GargoyleError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Validation error: {0}")]
    Validation(ValidationError),

    #[error("Not found: {entity_type} with id {id}")]
    NotFound { entity_type: String, id: String },

    #[error("Optimistic lock conflict: expected {expected}, found {found}")]
    LockConflict { expected: String, found: String },

    #[error("Schema error: {0}")]
    Schema(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationError {
    pub code: ErrorCode,
    pub field_path: String,
    pub message: String,
    pub expected: Option<String>,
    pub actual: Option<String>,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {} (field: {})", self.code, self.message, self.field_path)
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum ErrorCode {
    MissingRequiredField,
    InvalidFieldType,
    InvalidEnumValue,
    UnknownField,
    EntityRefTypeMismatch,
    EntityNotFound,
    EntityDeleted,
    LockConflict,
    InvalidStatusTransition,
    RelationTypeNotApproved,
    UnknownRelationType,
    UngroundedClaim,
    SchemaVersionMismatch,
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub type Result<T> = std::result::Result<T, GargoyleError>;

impl Serialize for GargoyleError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
