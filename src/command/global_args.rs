use std::path::PathBuf;
use clap::Args;

#[derive(Args, Debug)]
pub struct GlobalArgs {
    /// Path the the SqPack you wish to examine.
    pub repository: PathBuf,
}
