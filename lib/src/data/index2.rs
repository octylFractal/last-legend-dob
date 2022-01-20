use std::collections::HashMap;
use std::fs::File;
use std::io::Seek;
use std::path::PathBuf;

use binrw::{binread, helpers::count_with, io::SeekFrom};
use bitvec::prelude::*;

use crate::data::index_header::IndexHeader;
use crate::data::pack_header::PackHeader;
use crate::error::LastLegendError;
use crate::sqpath::SqPath;

#[binread]
#[derive(Debug)]
#[br(import { index_path: PathBuf })]
#[brw(little)]
pub struct Index2 {
    #[br(calc = index_path)]
    pub index_path: PathBuf,
    pub pack_header: PackHeader,
    pub index_header: IndexHeader,
    #[br(
        seek_before = SeekFrom::Start(index_header.index_data_offset.into()),
        parse_with = count_with(
            index_header.index_data_size.0 / ENTRY_SIZE,
            |reader, ro, args| {
                let entry = Index2Entry::read_options(reader, ro, args)?;
                Ok((entry.hash, entry))
            },
        ),
    )]
    pub entries: HashMap<u32, Index2Entry>,
}

impl Index2 {
    /// Get an entry for a [file].
    pub fn get_entry<F: AsRef<SqPath>>(&self, file: F) -> Result<&Index2Entry, LastLegendError> {
        let file = file.as_ref();
        self.entries.get(&file.sq_index_hash()).ok_or_else(|| {
            LastLegendError::MissingEntryFromIndex(file.to_owned(), self.index_path.clone())
        })
    }

    /// Given the [file] you want, open a reader and position it so it's ready to read a
    /// [DatEntryHeader] for the file.
    pub fn open_reader<F: AsRef<SqPath>>(&self, file: F) -> Result<File, LastLegendError> {
        let file = file.as_ref();
        self.shared_open_reader(file)
    }

    // Non-generic version for optimization.
    fn shared_open_reader(&self, file: &SqPath) -> Result<File, LastLegendError> {
        let entry = self.entries.get(&file.sq_index_hash()).ok_or_else(|| {
            LastLegendError::MissingEntryFromIndex(file.to_owned(), self.index_path.clone())
        })?;
        let path = self
            .index_path
            .parent()
            .expect("index path must have a parent")
            .join(
                self.index_path
                    .file_name()
                    .expect("index path must have a file name")
                    .to_string_lossy()
                    .replace(".index2", &*format!(".dat{}", entry.data_file_id)),
            );
        let mut reader = File::open(path)?;
        reader.seek(SeekFrom::Start(entry.offset_bytes.into()))?;
        Ok(reader)
    }
}

// Hash + info
const ENTRY_SIZE: usize = 4 + 4;

#[binread]
#[derive(Debug)]
#[brw(little)]
pub struct Index2Entry {
    pub hash: u32,
    #[br(temp, map = BitArray::new)]
    packed_info: BitArray<u32, Lsb0>,
    #[br(calc = packed_info[1..4].load_le::<u32>() >> 1)]
    pub data_file_id: u32,
    #[br(calc = packed_info[4..].load_le::<u32>() << 7)]
    pub offset_bytes: u32,
}
