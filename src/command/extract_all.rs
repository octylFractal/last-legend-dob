use std::borrow::Cow;
use std::path::{Path, PathBuf};

use clap::Args;

use last_legend_dob::data::repo::Repository;
use last_legend_dob::error::LastLegendError;
use last_legend_dob::sqpath::SqPathBuf;
use last_legend_dob::transformers::TransformerImpl;

use crate::command::extract_common::extract_entry;
use crate::command::global_args::GlobalArgs;
use crate::command::{make_open_options, LastLegendCommand};

/// Extract files from an index file.
#[derive(Args, Debug)]
pub struct ExtractAll {
    /// The index file to extract all from.
    files: Vec<PathBuf>,
    /// The extension to use for the output files.
    #[clap(short = 'e', long, default_value = "dat")]
    output_extension: String,
    /// Should errors be accepted?
    #[clap(short, long)]
    force_extract: bool,
    /// Should files be overwritten?
    #[clap(short, long)]
    overwrite: bool,
    /// Transformers to run
    #[clap(short, long)]
    transformer: Vec<TransformerImpl>,
}

impl LastLegendCommand for ExtractAll {
    fn run(mut self, global_args: GlobalArgs) -> Result<(), LastLegendError> {
        let output_open_options = make_open_options(self.overwrite);

        let repo = Repository::new(global_args.repository);

        self.files.sort();

        for file in self.files.into_iter() {
            let index = repo.load_index_file(Cow::Borrowed(file.as_path()))?;
            for entry in index.entries() {
                let entry_hash_hex = format!("{:X}", entry.hash);
                let res = extract_entry(
                    &repo,
                    SqPathBuf::new(&format!("{}.{}", entry_hash_hex, self.output_extension)),
                    Path::new(file.file_name().unwrap()).join(&entry_hash_hex),
                    &output_open_options,
                    &self.transformer,
                    &index,
                    entry,
                );
                if let Err(e) = res {
                    if self.force_extract {
                        eprintln!("Error extracting {}: {}", entry_hash_hex, e);
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Ok(())
    }
}
