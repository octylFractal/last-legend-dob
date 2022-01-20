use clap::Parser;

use last_legend_dob::error::LastLegendError;

use crate::command::{LastLegendCommand, LastLegendDob};

mod command;
mod uwu_colors;

fn main() -> Result<(), LastLegendError> {

    let args = LastLegendDob::parse();

    args.subcommand.run(args.global_args)
}
