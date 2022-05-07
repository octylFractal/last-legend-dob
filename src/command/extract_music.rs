use std::ffi::OsString;
use std::path::Path;

use clap::Args;
use owo_colors::Style;
use rayon::iter::{IntoParallelIterator, ParallelBridge, ParallelIterator};
use strum::EnumString;

use last_legend_dob::data::repo::Repository;
use last_legend_dob::error::LastLegendError;
use last_legend_dob::surpass::collection::Collection;
use last_legend_dob::surpass::known_rows::bgm::BGM;
use last_legend_dob::surpass::known_rows::orchestrion::Orchestrion;
use last_legend_dob::surpass::known_rows::orchestrion_path::OrchestrionPath;
use last_legend_dob::transformers::TransformerImpl;
use last_legend_dob::uwu_colors::ErrStyle;

use crate::command::extract_common::extract_file;
use crate::command::global_args::GlobalArgs;
use crate::command::{make_open_options, LastLegendCommand};

/// Extract all music files from the repository.
///
/// This can extract:
///
/// - All Orchestrion parts, with titles and comments. Uses `Orchestrion` and `OrchestrionPath` sheets.
///
/// - All baked-in music pieces, e.g. mount music. Uses `BGM` sheet.
#[derive(Args, Debug)]
pub struct ExtractMusic {
    /// Should files be overwritten?
    #[clap(short, long)]
    overwrite: bool,
    /// Music sources to include
    #[clap(short, long, required(true))]
    music_source: Vec<MusicSource>,
    /// Transformers to run
    #[clap(short, long)]
    transformer: Vec<TransformerImpl>,
}

impl LastLegendCommand for ExtractMusic {
    fn run(self, global_args: GlobalArgs) -> Result<(), LastLegendError> {
        let output_open_options = make_open_options(self.overwrite);

        let repo = Repository::new(global_args.repository);
        let collection = Collection::load(repo.clone())
            .map_err(|e| e.add_context("Failed to load collection"))?;

        let music_sources = self
            .music_source
            .into_iter()
            .map(|source| source.provide(&collection))
            .collect::<Result<Vec<_>, LastLegendError>>()?;
        music_sources
            .into_par_iter()
            .flat_map(|i| i.par_bridge())
            .try_for_each(|entry| -> Result<(), LastLegendError> {
                let (output_name, file) = entry?;
                if let Err(e) = extract_file(
                    &repo,
                    &file,
                    output_name,
                    &output_open_options,
                    &self.transformer,
                ) {
                    log::warn!(
                        "Failed to extract {}: {:#?}",
                        file.errstyle(Style::new().green()),
                        e
                    );
                }

                Ok(())
            })?;

        Ok(())
    }
}

#[derive(EnumString, Copy, Clone, Debug)]
#[strum(serialize_all = "snake_case")]
enum MusicSource {
    Bgm,
    Orchestrion,
}

type MusicSourceProvider =
    Box<dyn Iterator<Item = Result<(OsString, String), LastLegendError>> + Send>;

impl MusicSource {
    fn provide(&self, collection: &Collection) -> Result<MusicSourceProvider, LastLegendError> {
        let iter: MusicSourceProvider = match self {
            Self::Bgm => Box::new(
                collection
                    .sheet_iter("BGM")?
                    .deserialize_rows::<BGM>()
                    .filter_map(|row| {
                        let row = match row {
                            Ok(v) => v,
                            Err(e) => return Some(Err(e)),
                        };
                        (!row.file.is_empty()).then(|| {
                            let base_name =
                                Path::new(&row.file).file_stem().unwrap().to_os_string();
                            Ok((base_name, row.file))
                        })
                    }),
            ),
            Self::Orchestrion => {
                let orch_paths: Vec<String> = collection
                    .sheet_iter("OrchestrionPath")?
                    .deserialize_rows::<OrchestrionPath>()
                    .map(|r| r.map(|o| o.file_name))
                    .collect::<Result<_, LastLegendError>>()?;
                Box::new(
                    collection
                        .sheet_iter("Orchestrion")?
                        .deserialize_rows::<Orchestrion>()
                        .enumerate()
                        .filter_map(move |(i, row)| {
                            let row = match row {
                                Ok(v) => v,
                                Err(e) => return Some(Err(e)),
                            };
                            (!row.name.is_empty()).then(|| {
                                let orch_path = String::from(&orch_paths[i]);
                                let extract_name = Path::new(&orch_path).with_file_name(row.name);
                                Ok((extract_name.into_os_string(), orch_path))
                            })
                        }),
                )
            }
        };
        Ok(iter)
    }
}
