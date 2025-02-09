use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Seek};
use std::path::{Path, PathBuf};

use binrw::{binread, helpers::count_with, io::SeekFrom, BinReaderExt};
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
    pub fn load<P: AsRef<Path>, F: AsRef<SqPath>>(
        repo_path: P,
        file: F,
    ) -> Result<Self, LastLegendError> {
        let repo_path = repo_path.as_ref();
        let file = file.as_ref();
        let index_path = file
            .sqpack_index_path(repo_path)
            .ok_or_else(|| LastLegendError::InvalidSqPath(file.as_str().to_string()))?;

        Self::load_from_path(index_path)
    }

    pub fn load_from_path<P: AsRef<Path>>(index_path: P) -> Result<Self, LastLegendError> {
        let index_path = index_path.as_ref();
        let mut reader = BufReader::new(
            File::open(index_path)
                .map_err(|e| LastLegendError::Io("Couldn't open reader".into(), e))?,
        );

        reader
            .read_le_args::<Index2>(
                Index2BinReadArgs::builder()
                    .index_path(index_path.to_path_buf())
                    .finalize(),
            )
            .map_err(|e| LastLegendError::BinRW("Couldn't read Index2".into(), e))
    }

    pub fn entries(&self) -> impl Iterator<Item = &Index2Entry> {
        self.entries.values()
    }

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
        self.open_reader_for_entry(self.get_entry(file)?)
    }

    pub fn open_reader_for_entry(&self, entry: &Index2Entry) -> Result<File, LastLegendError> {
        let path = self
            .index_path
            .parent()
            .expect("index path must have a parent")
            .join(
                self.index_path
                    .file_name()
                    .expect("index path must have a file name")
                    .to_string_lossy()
                    .replace(".index2", &format!(".dat{}", entry.data_file_id)),
            );
        let mut reader =
            File::open(path).map_err(|e| LastLegendError::Io("Couldn't open reader".into(), e))?;
        reader
            .seek(SeekFrom::Start(entry.offset_bytes))
            .map_err(|e| LastLegendError::Io("Couldn't seek into reader".into(), e))?;
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
    #[br(calc = packed_info[1..4].load_le::<u32>())]
    pub data_file_id: u32,
    #[br(calc = (u64::from(packed_info[4..].load_le::<u32>())) << 7)]
    pub offset_bytes: u64,
}
