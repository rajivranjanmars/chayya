use std::fmt;
use warp::reject::Reject;

#[derive(Debug)]
pub enum AppError {
    DatabaseError(String),
    NotFound(String),
    ValidationError(String),
    TemplateError(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AppError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            AppError::TemplateError(msg) => write!(f, "Template error: {}", msg),
        }
    }
}

impl Reject for AppError {}
