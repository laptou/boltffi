use boltffi::*;

use crate::records::blittable::DataPoint;

/// Errors that can happen during math operations.
#[error]
#[derive(Clone, Debug, PartialEq)]
pub enum MathError {
    DivisionByZero,
    NegativeInput,
    Overflow,
}

impl std::fmt::Display for MathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DivisionByZero => write!(f, "division by zero"),
            Self::NegativeInput => write!(f, "negative input"),
            Self::Overflow => write!(f, "overflow"),
        }
    }
}

impl std::error::Error for MathError {}

impl From<UnexpectedFfiCallbackError> for MathError {
    fn from(_: UnexpectedFfiCallbackError) -> Self {
        Self::Overflow
    }
}

#[export]
pub fn checked_divide(a: i32, b: i32) -> Result<i32, MathError> {
    if b == 0 {
        Err(MathError::DivisionByZero)
    } else {
        Ok(a / b)
    }
}

#[export]
pub fn checked_sqrt(x: f64) -> Result<f64, MathError> {
    if x < 0.0 {
        Err(MathError::NegativeInput)
    } else {
        Ok(x.sqrt())
    }
}

#[export]
pub fn checked_add(a: i32, b: i32) -> Result<i32, MathError> {
    a.checked_add(b).ok_or(MathError::Overflow)
}

#[error]
#[derive(Clone, Debug, PartialEq)]
pub struct AppError {
    pub code: i32,
    pub message: String,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.message, self.code)
    }
}

impl std::error::Error for AppError {}

#[export]
pub fn may_fail(valid: bool) -> Result<String, AppError> {
    if valid {
        Ok("Success!".to_string())
    } else {
        Err(AppError {
            code: 400,
            message: "Invalid input".to_string(),
        })
    }
}

#[export]
pub fn divide_app(a: i32, b: i32) -> Result<i32, AppError> {
    if b == 0 {
        Err(AppError {
            code: 500,
            message: "Division by zero".to_string(),
        })
    } else {
        Ok(a / b)
    }
}

#[error]
#[derive(Clone, Debug, PartialEq)]
#[repr(i32)]
pub enum ValidationError {
    TooShort = 1,
    TooLong = 2,
    InvalidFormat = 3,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort => write!(f, "too short"),
            Self::TooLong => write!(f, "too long"),
            Self::InvalidFormat => write!(f, "invalid format"),
        }
    }
}

impl std::error::Error for ValidationError {}

/// Validates a username against length and format rules.
///
/// Returns the username on success, or a typed ValidationError
/// that tells the caller exactly what went wrong.
#[export]
pub fn validate_username(name: String) -> Result<String, ValidationError> {
    if name.len() < 3 {
        Err(ValidationError::TooShort)
    } else if name.len() > 20 {
        Err(ValidationError::TooLong)
    } else if name.contains(' ') {
        Err(ValidationError::InvalidFormat)
    } else {
        Ok(name)
    }
}

#[data]
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(i32)]
pub enum ApiResult {
    Success = 0,
    ErrorCode(i32) = 1,
    ErrorWithData { code: i32, detail: i32 } = 2,
}

#[error]
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(i32)]
pub enum ComputeError {
    InvalidInput(i32) = 0,
    Overflow { value: i32, limit: i32 } = 1,
}

impl std::fmt::Display for ComputeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput(value) => write!(f, "invalid input: {}", value),
            Self::Overflow { value, limit } => {
                write!(f, "overflow: value {} exceeds limit {}", value, limit)
            }
        }
    }
}

impl std::error::Error for ComputeError {}

#[data]
#[derive(Clone, Debug, PartialEq)]
pub struct BenchmarkResponse {
    pub request_id: i64,
    pub result: Result<DataPoint, ComputeError>,
}

#[export]
pub fn process_value(value: i32) -> ApiResult {
    if value > 0 {
        ApiResult::Success
    } else if value == 0 {
        ApiResult::ErrorCode(-1)
    } else {
        ApiResult::ErrorWithData {
            code: value,
            detail: value * 2,
        }
    }
}

#[export]
pub fn api_result_is_success(result: ApiResult) -> bool {
    matches!(result, ApiResult::Success)
}

#[export]
pub fn try_compute(value: i32) -> Result<i32, ComputeError> {
    if value > 0 {
        Ok(value * 2)
    } else if value == 0 {
        Err(ComputeError::InvalidInput(-999))
    } else {
        Err(ComputeError::Overflow { value, limit: 0 })
    }
}

#[export]
pub fn create_success_response(request_id: i64, point: DataPoint) -> BenchmarkResponse {
    BenchmarkResponse {
        request_id,
        result: Ok(point),
    }
}

#[export]
pub fn create_error_response(request_id: i64, error: ComputeError) -> BenchmarkResponse {
    BenchmarkResponse {
        request_id,
        result: Err(error),
    }
}

#[export]
pub fn is_response_success(response: BenchmarkResponse) -> bool {
    response.result.is_ok()
}

#[export]
pub fn get_response_value(response: BenchmarkResponse) -> Option<DataPoint> {
    response.result.ok()
}
