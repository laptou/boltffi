/// errors produced while lowering ffi contracts into language-specific render plans.
#[derive(Debug, thiserror::Error)]
pub enum LowerError {
    /// ffi and abi contracts disagree (missing wire ops, missing call bindings, etc.).
    #[error("invalid: {0}")]
    Invalid(String),

    /// the contract uses a shape this backend does not implement yet.
    #[error("unsupported: {0}")]
    Unsupported(String),
}
