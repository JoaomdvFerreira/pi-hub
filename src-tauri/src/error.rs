use serde::Serialize;

#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct ApplicationError {
    pub code: String,
    pub message: String,
    pub remediation: Option<String>,
    pub retryable: bool,
}
