use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationError {
    pub code: String,
    pub message: String,
    pub remediation: Option<String>,
    pub retryable: bool,
}
