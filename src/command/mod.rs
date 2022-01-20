use clap::{Parser, Subcommand};

use last_legend_dob::error::LastLegendError;

use crate::command::global_args::GlobalArgs;

mod extract;
mod global_args;

pub trait LastLegendCommand {
    fn run(self, global_args: GlobalArgs) -> Result<(), LastLegendError>;
}

#[derive(Parser, Debug)]
#[clap(about = "FFXIV file extractor", version)]
pub struct LastLegendDob {
    #[clap(flatten)]
    pub global_args: GlobalArgs,
    /// Thing to do.
    #[clap(subcommand)]
    pub subcommand: LLDCommand,
}

#[derive(Subcommand, Debug)]
pub enum LLDCommand {
    Extract(extract::Extract),
}

impl LastLegendCommand for LLDCommand {
    fn run(self, global_args: GlobalArgs) -> Result<(), LastLegendError> {
        match self {
            LLDCommand::Extract(v) => v.run(global_args),
        }
    }
}
