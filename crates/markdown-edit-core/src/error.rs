use std::io;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExitCode {
    Success = 0,
    NotFound = 1,
    TooManyMatches = 2,
    InvalidArguments = 3,
    Io = 4,
    InvalidContent = 5,
    Validation = 6,
}

impl ExitCode {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Success),
            1 => Some(Self::NotFound),
            2 => Some(Self::TooManyMatches),
            3 => Some(Self::InvalidArguments),
            4 => Some(Self::Io),
            5 => Some(Self::InvalidContent),
            6 => Some(Self::Validation),
            _ => None,
        }
    }
}

#[derive(Debug, Error)]
pub enum EditError {
    #[error("no matching sections found")]
    NotFound,

    #[error("number of matches exceeds maximum allowed ({max})")]
    TooManyMatches { max: usize, actual: usize },

    #[error("invalid arguments: {0}")]
    InvalidArguments(String),

    #[error("i/o error: {0}")]
    Io(#[from] io::Error),

    #[error("invalid content source: {0}")]
    InvalidContent(String),

    #[error("validation failed: {0}")]
    Validation(String),
}

impl EditError {
    pub fn exit_code(&self) -> ExitCode {
        match self {
            Self::NotFound => ExitCode::NotFound,
            Self::TooManyMatches { .. } => ExitCode::TooManyMatches,
            Self::InvalidArguments(_) => ExitCode::InvalidArguments,
            Self::Io { .. } => ExitCode::Io,
            Self::InvalidContent(_) => ExitCode::InvalidContent,
            Self::Validation(_) => ExitCode::Validation,
        }
    }
}

pub type EditResult<T> = Result<T, EditError>;
