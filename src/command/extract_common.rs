use last_legend_dob::data::index2::{Index2, Index2Entry};
use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::path::Path;
use std::sync::Arc;

use last_legend_dob::data::repo::Repository;
use last_legend_dob::error::LastLegendError;
use last_legend_dob::simple_task::format_index_entry_for_console;
use last_legend_dob::simple_task::{create_transformed_reader, TransformedReader};
use last_legend_dob::sqpath::{SqPath, SqPathBuf};
use last_legend_dob::transformers::TransformerImpl;

pub(crate) fn extract_file<F: AsRef<SqPath>, O: AsRef<OsStr>>(
    repo: &Repository,
    file: F,
    output_base_name: O,
    output_open_options: &OpenOptions,
    transformers: &[TransformerImpl],
) -> Result<(), LastLegendError> {
    let file = file.as_ref();
    let index = repo.get_index_for(file)?;
    let entry = index.get_entry(file)?;

    extract_entry(
        repo,
        file.to_owned(),
        output_base_name,
        output_open_options,
        transformers,
        &index,
        entry,
    )
}

pub(crate) fn extract_entry<O: AsRef<OsStr>>(
    repo: &Repository,
    file_name: SqPathBuf,
    output_base_name: O,
    output_open_options: &OpenOptions,
    transformers: &[TransformerImpl],
    index: &Arc<Index2>,
    entry: &Index2Entry,
) -> Result<(), LastLegendError> {
    log::info!(
        "Extracting {}...",
        format_index_entry_for_console(repo.repo_path(), index, entry, &file_name)
    );
    let TransformedReader {
        file_name,
        mut reader,
    } = create_transformed_reader(index, entry, file_name, transformers)?;

    let output_path = Path::new(&output_base_name)
        .with_extension(Path::new(file_name.as_str()).extension().unwrap());
    std::fs::create_dir_all(output_path.parent().unwrap())
        .map_err(|e| LastLegendError::Io("Couldn't create output dirs".into(), e))?;
    let mut output = output_open_options
        .open(output_path)
        .map_err(|e| LastLegendError::Io("Couldn't open output".into(), e))?;
    std::io::copy(&mut reader, &mut output)
        .map_err(|e| LastLegendError::Io("Couldn't write output".into(), e))?;

    log::info!("Done!");

    Ok(())
}
