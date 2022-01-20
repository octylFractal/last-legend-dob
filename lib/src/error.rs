use std::path::PathBuf;

use thiserror::Error;

use crate::sqpath::SqPathBuf;

#[derive(Error, Debug)]
pub enum LastLegendError {
    #[error("Invalid SqPath given: {0}")]
    InvalidSqPath(String),
    #[error("Entry '{0}' is not its index file '{1}'")]
    MissingEntryFromIndex(SqPathBuf, PathBuf),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("binrw error: {0}")]
    BinRW(#[from] binrw::Error),
}
