use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    RateLimit,
    ContextOverflow,
    Authentication,
    ServerError,
    NetworkError,
    ContentFilter,
    ModelNotFound,
    QuotaExceeded,
    Unknown,
}

#[derive(Clone, Debug, Serialize)]
pub struct ClassifiedError {
    pub category: ErrorCategory,
    pub status_code: Option<u16>,
    pub message: String,
    pub retryable: bool,
    pub suggested_action: String,
}

pub fn classify_error(status_code: u16, body: &str) -> ClassifiedError {
    let body_lower = body.to_lowercase();

    match status_code {
        429 => ClassifiedError {
            category: ErrorCategory::RateLimit,
            status_code: Some(status_code),
            message: "Rate limit exceeded".to_string(),
            retryable: true,
            suggested_action: "Wait and retry with exponential backoff".to_string(),
        },
        400 => {
            if body_lower.contains("context_length")
                || body_lower.contains("max_tokens")
                || body_lower.contains("token limit")
                || body_lower.contains("too many tokens")
            {
                ClassifiedError {
                    category: ErrorCategory::ContextOverflow,
                    status_code: Some(status_code),
                    message: "Context length exceeded".to_string(),
                    retryable: false,
                    suggested_action: "Compress context or reduce message count".to_string(),
                }
            } else if body_lower.contains("content_policy")
                || body_lower.contains("content_filter")
                || body_lower.contains("safety")
            {
                ClassifiedError {
                    category: ErrorCategory::ContentFilter,
                    status_code: Some(status_code),
                    message: "Content filtered by provider".to_string(),
                    retryable: false,
                    suggested_action: "Rephrase the request to avoid filtered content".to_string(),
                }
            } else if body_lower.contains("model_not_found")
                || body_lower.contains("does not exist")
                || body_lower.contains("invalid model")
            {
                ClassifiedError {
                    category: ErrorCategory::ModelNotFound,
                    status_code: Some(status_code),
                    message: "Model not found".to_string(),
                    retryable: false,
                    suggested_action: "Check model name and provider".to_string(),
                }
            } else {
                ClassifiedError {
                    category: ErrorCategory::Unknown,
                    status_code: Some(status_code),
                    message: body.chars().take(200).collect(),
                    retryable: false,
                    suggested_action: "Check request format".to_string(),
                }
            }
        }
        401 | 403 => ClassifiedError {
            category: ErrorCategory::Authentication,
            status_code: Some(status_code),
            message: "Authentication failed".to_string(),
            retryable: false,
            suggested_action: "Check API key and permissions".to_string(),
        },
        404 => ClassifiedError {
            category: ErrorCategory::ModelNotFound,
            status_code: Some(status_code),
            message: "Endpoint or model not found".to_string(),
            retryable: false,
            suggested_action: "Verify API URL and model name".to_string(),
        },
        500 | 502 => ClassifiedError {
            category: ErrorCategory::ServerError,
            status_code: Some(status_code),
            message: "Server error".to_string(),
            retryable: true,
            suggested_action: "Provider issue, retry with backoff".to_string(),
        },
        503 => ClassifiedError {
            category: ErrorCategory::ServerError,
            status_code: Some(status_code),
            message: "Service unavailable".to_string(),
            retryable: true,
            suggested_action: "Provider overloaded, retry with backoff".to_string(),
        },
        529 => ClassifiedError {
            category: ErrorCategory::RateLimit,
            status_code: Some(status_code),
            message: "Provider overloaded".to_string(),
            retryable: true,
            suggested_action: "Provider overloaded, retry with backoff".to_string(),
        },
        _ => {
            if body_lower.contains("quota")
                || body_lower.contains("billing")
                || body_lower.contains("insufficient balance")
            {
                ClassifiedError {
                    category: ErrorCategory::QuotaExceeded,
                    status_code: Some(status_code),
                    message: "Quota or credits exhausted".to_string(),
                    retryable: false,
                    suggested_action: "Check provider billing and add credits".to_string(),
                }
            } else {
                ClassifiedError {
                    category: ErrorCategory::Unknown,
                    status_code: Some(status_code),
                    message: body.chars().take(200).collect(),
                    retryable: false,
                    suggested_action: "Unclassified error".to_string(),
                }
            }
        }
    }
}

pub fn is_retryable(status_code: u16, body: &str) -> bool {
    classify_error(status_code, body).retryable
}
