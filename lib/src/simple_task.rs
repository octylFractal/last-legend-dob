use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom};
use std::path::Path;

use binrw::BinReaderExt;
use owo_colors::{Style, Styled};

use crate::data::dat::DatEntryHeader;
use crate::data::index2::{Index2, Index2Entry};
use crate::error::LastLegendError;
use crate::sqpath::{SqPath, SqPathBuf};
use crate::transformers::{Transformer, TransformerForFile, TransformerImpl};
use crate::uwu_colors::{get_errstyle, ErrStyle};

pub fn read_file_entry_header<F: AsRef<SqPath>>(
    index: &Index2,
    file: F,
) -> Result<(DatEntryHeader, BufReader<File>), LastLegendError> {
    let entry = index.get_entry(file)?;

    read_entry_header(index, entry)
}

fn read_entry_header(
    index: &Index2,
    entry: &Index2Entry,
) -> Result<(DatEntryHeader, BufReader<File>), LastLegendError> {
    let mut dat_reader = BufReader::new(index.open_reader_for_entry(entry)?);
    let original_pos = dat_reader
        .stream_position()
        .map_err(|e| LastLegendError::Io("Couldn't read dat_reader stream pos".into(), e))?;
    let header: DatEntryHeader = dat_reader
        .read_le()
        .map_err(|e| LastLegendError::BinRW("Couldn't read DatEntryHeader".into(), e))?;
    dat_reader
        .seek(SeekFrom::Start(original_pos))
        .map_err(|e| LastLegendError::Io("Couldn't seek to original dat_reader pos".into(), e))?;

    Ok((header, dat_reader))
}

/// Create a reader for the data after applying transforms.
pub fn create_transformed_reader(
    index: &Index2,
    entry: &Index2Entry,
    mut file_name: SqPathBuf,
    transformers: &[TransformerImpl],
) -> Result<TransformedReader, LastLegendError> {
    let (header, dat_reader) = read_entry_header(index, entry)?;

    let content = header
        .read_content_to_vec(dat_reader)
        .map_err(|e| LastLegendError::Io("Failed to read dat content".into(), e))?;

    let mut reader: Box<dyn Read + Send> = Box::new(Cursor::new(content));
    for t in transformers {
        if let Some(tf) = t.maybe_for(file_name.clone()) {
            file_name = tf.renamed_file().into_owned();
            reader = tf.transform(reader)?;
        }
    }

    Ok(TransformedReader { file_name, reader })
}

pub struct TransformedReader {
    pub file_name: SqPathBuf,
    pub reader: Box<dyn Read + Send>,
}

pub fn format_index_entry_for_console<P: AsRef<Path>, F: AsRef<SqPath>>(
    repo_path: P,
    index: &Index2,
    entry: &Index2Entry,
    file: F,
) -> String {
    let repo_path = repo_path.as_ref();
    let file = file.as_ref();
    format!(
        "{} ({}), in index file {}, in data file {}, at offset {}",
        file.errstyle(Style::new().green()),
        format_index_hash_for_console(entry.hash),
        index
            .index_path
            .strip_prefix(repo_path)
            .expect("Index path should start with the repository path")
            .display()
            .errstyle(Style::new().yellow()),
        entry.data_file_id.errstyle(Style::new().yellow()),
        format!("0x{:X}", entry.offset_bytes).errstyle(Style::new().yellow()),
    )
}

pub fn format_index_hash_for_console(hash: u32) -> Styled<String> {
    get_errstyle(Style::new().blue()).style(format!("0x{:X}", hash))
}
