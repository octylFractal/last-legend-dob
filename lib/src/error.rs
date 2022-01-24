use std::path::PathBuf;

use thiserror::Error;

use crate::sqpath::SqPathBuf;

#[derive(Error, Debug)]
pub enum LastLegendError {
    #[error("Invalid SqPath given: {0}")]
    InvalidSqPath(String),
    #[error("Entry '{0}' is not its index file '{1}'")]
    MissingEntryFromIndex(SqPathBuf, PathBuf),
    #[error("I/O error: {0}, {1}")]
    Io(String, #[source] std::io::Error),
    #[error("binrw error: {0}, {1}")]
    BinRW(String, #[source] binrw::Error),
    #[error("FFMPEG failed: {0}")]
    FFMPEG(String),
}
