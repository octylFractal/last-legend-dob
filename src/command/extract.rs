use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom};
use std::path::PathBuf;

use binrw::BinReaderExt;
use clap::Args;
use itertools::Itertools;
use owo_colors::Style;

use last_legend_dob::data::dat::DatEntryHeader;
use last_legend_dob::data::index2::{Index2, Index2BinReadArgs};
use last_legend_dob::error::LastLegendError;
use last_legend_dob::error::LastLegendError::InvalidSqPath;
use last_legend_dob::sqpath::{SqPath, SqPathBuf};
use last_legend_dob::transformers::TransformerImpl;

use crate::command::global_args::GlobalArgs;
use crate::command::LastLegendCommand;
use crate::uwu_colors::ErrStyle;

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
    transformers: Vec<TransformerImpl>,
}

impl LastLegendCommand for Extract {
    fn run(self, global_args: GlobalArgs) -> Result<(), LastLegendError> {
        let output_open_options = {
            let mut opts = std::fs::File::options();
            opts.create(true)
                .write(true)
                .truncate(true)
                .create_new(!self.overwrite);
            opts
        };

        let file_to_index: HashMap<SqPathBuf, PathBuf> = self
            .files
            .iter()
            .map(|f| {
                f.sqpack_index_path(&global_args.repository)
                    .ok_or_else(|| InvalidSqPath(f.as_str().to_string()))
                    .map(|index_path| (f.clone(), index_path))
            })
            .try_collect()?;

        let indexes: HashMap<PathBuf, Index2> = file_to_index
            .values()
            .into_iter()
            .unique()
            .map(|index_path| {
                let mut reader = BufReader::new(
                    std::fs::File::open(index_path)
                        .map_err(|e| LastLegendError::Io("Couldn't open reader".into(), e))?,
                );

                reader
                    .read_le_args::<Index2>(
                        Index2BinReadArgs::builder()
                            .index_path(index_path.clone())
                            .finalize(),
                    )
                    .map_err(|e| LastLegendError::BinRW("Couldn't read Index2".into(), e))
                    .map(|index2| (index_path.clone(), index2))
            })
            .try_collect()?;

        let file_to_index: HashMap<SqPathBuf, &Index2> = file_to_index
            .into_iter()
            .map(|(file, index_path)| (file, &indexes[&index_path]))
            .collect();

        for (file, index) in file_to_index
            .into_iter()
            .sorted_by_key(|(file, _)| file.to_owned())
        {
            let entry = index.get_entry(&file)?;
            log::info!(
                "Extracting {} ({}), in index file {}, in data file {}, at offset {}...",
                file.errstyle(Style::new().green()),
                format!("0x{:X}", file.sq_index_hash()).errstyle(Style::new().blue()),
                index
                    .index_path
                    .strip_prefix(&global_args.repository)
                    .expect("Index path should start with the repository path")
                    .display()
                    .errstyle(Style::new().yellow()),
                entry.data_file_id.errstyle(Style::new().yellow()),
                format!("0x{:X}", entry.offset_bytes).errstyle(Style::new().yellow()),
            );
            fallible_copy(
                &self.transformers,
                output_open_options.clone(),
                &file,
                index,
            )?;
            log::info!("Done!");
        }

        Ok(())
    }
}

fn fallible_copy(
    transformers: &[TransformerImpl],
    output_open_options: OpenOptions,
    file: &SqPath,
    index: &Index2,
) -> Result<(), LastLegendError> {
    let mut dat_reader = BufReader::new(index.open_reader(file)?);
    let original_pos = dat_reader
        .stream_position()
        .map_err(|e| LastLegendError::Io("Couldn't read dat_reader stream pos".into(), e))?;
    let header: DatEntryHeader = dat_reader
        .read_le()
        .map_err(|e| LastLegendError::BinRW("Couldn't read DatEntryHeader".into(), e))?;
    dat_reader
        .seek(SeekFrom::Start(original_pos))
        .map_err(|e| LastLegendError::Io("Couldn't seek to original dat_reader pos".into(), e))?;

    let mut content = Vec::with_capacity(header.uncompressed_size.try_into().unwrap());
    header
        .read_content(dat_reader)
        .map_err(|e| LastLegendError::Io("Couldn't read content".into(), e))?
        .read_to_end(&mut content)
        .map_err(|e| LastLegendError::Io("Couldn't cache content".into(), e))?;
    assert_eq!(
        usize::try_from(header.uncompressed_size).unwrap(),
        content.len()
    );

    let mut file_name: SqPathBuf = file.to_owned();
    let mut reader: Box<dyn Read> = Box::new(Cursor::new(content));
    for t in transformers {
        let t = t.into_boxed_transformer();
        if t.can_transform(&file_name) {
            file_name = t.rename_file(&file_name).into_owned();
            reader = t.transform(&file_name, reader)?;
        }
    }

    let output_path = PathBuf::from(file_name.as_str());
    std::fs::create_dir_all(output_path.parent().unwrap())
        .map_err(|e| LastLegendError::Io("Couldn't create output dirs".into(), e))?;
    let mut output = output_open_options
        .open(output_path)
        .map_err(|e| LastLegendError::Io("Couldn't open output".into(), e))?;
    std::io::copy(&mut reader, &mut output)
        .map_err(|e| LastLegendError::Io("Couldn't write output".into(), e))?;
    Ok(())
}
