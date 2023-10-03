use std::fs::OpenOptions;

use clap::{Parser, Subcommand};

use last_legend_dob::error::LastLegendError;
use last_legend_dob::simple_task::format_index_hash_for_console;
use last_legend_dob::sqpath::SqPathBuf;

use crate::command::global_args::GlobalArgs;

mod extract;
mod extract_all;
pub(crate) mod extract_common;
mod extract_music;
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
    ExtractAll(extract_all::ExtractAll),
    ExtractMusic(extract_music::ExtractMusic),
    /// Get the hash of the path, used to retrieve data from the index.
    HashPath {
        /// Path to compute the hash for.
        path: SqPathBuf,
    },
}

impl LastLegendCommand for LLDCommand {
    fn run(self, global_args: GlobalArgs) -> Result<(), LastLegendError> {
        match self {
            Self::Extract(v) => v.run(global_args),
            Self::ExtractAll(v) => v.run(global_args),
            Self::ExtractMusic(v) => v.run(global_args),
            Self::HashPath { path } => {
                log::info!(
                    "Hash of path is {}",
                    format_index_hash_for_console(path.sq_index_hash())
                );
                Ok(())
            }
        }
    }
}

pub(crate) fn make_open_options(overwrite: bool) -> OpenOptions {
    let mut opts = std::fs::File::options();
    opts.create(true)
        .write(true)
        .truncate(true)
        .create_new(!overwrite);
    opts
}
