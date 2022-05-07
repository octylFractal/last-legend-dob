use std::fmt::{Debug, Display};
use std::path::PathBuf;

use thiserror::Error;

use crate::sqpath::SqPathBuf;

#[derive(Error, Debug)]
pub enum LastLegendError {
    #[error("Invalid SqPath given: {0}")]
    InvalidSqPath(String),
    #[error("Entry '{0}' is not its index file '{1}'")]
    MissingEntryFromIndex(SqPathBuf, PathBuf),
    #[error("Collection sheet line is invalid: {0}")]
    CollectionSheetLineInvalid(String),
    #[error("Sheet name is invalid: {0}")]
    SheetNameInvalid(String),
    #[error("{0}")]
    Custom(String),
    #[error("Additional context for error: {0}, {1}")]
    LastLegend(String, #[source] Box<LastLegendError>),
    #[error("I/O error: {0}, {1}")]
    Io(String, #[source] std::io::Error),
    #[error("binrw error: {0}, {1}")]
    BinRW(String, #[source] binrw::Error),
    #[error("FFMPEG failed: {0}")]
    FFMPEG(String),
}

impl serde::de::Error for LastLegendError {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        LastLegendError::Custom(msg.to_string())
    }
}

impl LastLegendError {
    pub fn add_context(self, message: impl Into<String>) -> Self {
        Self::LastLegend(message.into(), Box::new(self))
    }
}
