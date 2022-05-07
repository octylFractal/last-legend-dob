use clap::Args;
use std::path::Path;

use last_legend_dob::data::repo::Repository;
use last_legend_dob::error::LastLegendError;
use last_legend_dob::sqpath::SqPathBuf;
use last_legend_dob::transformers::TransformerImpl;

use crate::command::extract_common::extract_file;
use crate::command::global_args::GlobalArgs;
use crate::command::{make_open_options, LastLegendCommand};

/// Extract files from the repository.
#[derive(Args, Debug)]
pub struct Extract {
    /// The files to extract
    files: Vec<SqPathBuf>,
    /// Should files be overwritten?
    #[clap(short, long)]
    overwrite: bool,
    /// Transformers to run
    #[clap(short, long)]
    transformer: Vec<TransformerImpl>,
}

impl LastLegendCommand for Extract {
    fn run(mut self, global_args: GlobalArgs) -> Result<(), LastLegendError> {
        let output_open_options = make_open_options(self.overwrite);

        let repo = Repository::new(global_args.repository);

        self.files.sort();

        for file in self.files.into_iter() {
            let base_name = Path::new(file.as_str()).file_stem().unwrap();
            extract_file(
                &repo,
                &file,
                base_name,
                &output_open_options,
                &self.transformer,
            )?;
        }

        Ok(())
    }
}
