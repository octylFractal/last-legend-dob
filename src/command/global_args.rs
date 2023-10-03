use clap::Args;
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct GlobalArgs {
    /// Path the the SqPack you wish to examine.
    pub repository: PathBuf,
    /// Verbosity level, repeat to increase.
    #[clap(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}
