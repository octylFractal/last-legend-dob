use clap::Parser;
use log::LevelFilter;

use last_legend_dob::error::LastLegendError;

use crate::command::{LastLegendCommand, LastLegendDob};

mod command;

fn main() -> Result<(), LastLegendError> {
    let args = LastLegendDob::parse();
    env_logger::Builder::new()
        .filter_level(match args.global_args.verbose {
            0 => LevelFilter::Info,
            1 => LevelFilter::Debug,
            _ => LevelFilter::Trace,
        })
        .init();

    args.subcommand.run(args.global_args)
}
