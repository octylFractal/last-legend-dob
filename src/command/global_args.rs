use clap::Args;
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct GlobalArgs {
    /// Path the the SqPack you wish to examine.
    pub repository: PathBuf,
}
