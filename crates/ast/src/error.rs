use alloy_primitives::{hex, ruint};
use thiserror::Error as ThisError;

#[derive(ThisError, Debug, PartialEq)]
pub enum Error {
    #[error("{0}")]
    WordOverflow(#[from] ruint::ParseError),

    #[error("{0}")]
    BytesOddLength(#[from] hex::FromHexError),

    #[error("{0}")]
    InvalidSolType(#[from] alloy_dyn_abi::Error),

    /// Placeholder
    #[error("TODO: {0}")]
    Todo(String),
}
