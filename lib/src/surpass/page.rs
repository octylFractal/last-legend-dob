use crate::error::LastLegendError;
use crate::surpass::sheet_info::{SheetInfo, Variant};
use binrw::{binread, BinReaderExt};
use std::io::{Read, Seek, SeekFrom};

const ROW_OFFSET_SIZE: u32 = 8;

#[binread]
#[derive(Debug)]
// Magic includes version
#[br(big, magic = b"EXDF\0\x02")]
pub struct PageHeader {
    #[br(temp)]
    _unknown_1: [u8; 2],
    #[br(temp)]
    offset_table_size: u32,
    #[br(temp)]
    _unknown_2: [u8; 20],
    #[br(args { count: (offset_table_size / ROW_OFFSET_SIZE).try_into().unwrap() })]
    offset_table: Vec<RowOffset>,
}

impl PageHeader {
    /// Get an iterator over the row buffers, to be parsed into actual structs at a higher level.
    pub fn row_buffer_iter<R: Read + Seek + Send>(
        &self,
        reader: R,
        sheet_info: &SheetInfo,
    ) -> RowBufferIter<R> {
        RowBufferIter {
            reader,
            fixed_row_size: sheet_info.fixed_row_size.into(),
            row_offsets: self.offset_table.iter().map(|t| t.offset.into()).collect(),
            row_offset_index: 0,
            sub_row: match sheet_info.variant {
                Variant::Default => SubRow::None,
                Variant::SubRows => SubRow::Inactive,
            },
        }
    }
}

#[binread]
#[derive(Debug)]
pub struct RowOffset {
    pub index: u32,
    pub offset: u32,
}

pub struct RowBufferIter<R> {
    reader: R,
    fixed_row_size: u64,
    row_offsets: Vec<u64>,
    row_offset_index: usize,
    sub_row: SubRow,
}

enum SubRow {
    None,
    Inactive,
    Active(Box<dyn Iterator<Item = u64> + Send>),
}

const ROW_HEADER_SIZE: u64 = 6;

impl<R: Read + Seek> RowBufferIter<R> {
    pub fn into_reader(self) -> R {
        self.reader
    }

    fn read_row_header(reader: &mut R) -> Result<(u32, u16), LastLegendError> {
        reader
            .read_be()
            .map_err(|e| LastLegendError::BinRW("Failed to read row header".into(), e))
    }

    fn next_row_offset(&mut self) -> Option<u64> {
        (self.row_offset_index < self.row_offsets.len()).then(|| {
            let v = self.row_offsets[self.row_offset_index];
            self.row_offset_index += 1;
            v
        })
    }

    fn default_iter(reader: &mut R, offset: u64) -> <Self as Iterator>::Item {
        reader
            .seek(SeekFrom::Start(offset))
            .map_err(|e| LastLegendError::Io("Failed to seek to row".into(), e))?;
        let (data_size, count) = Self::read_row_header(reader)?;
        assert_eq!(count, 1, "default row should always be count == 1");

        reader
            .bytes()
            .take(data_size as usize)
            .collect::<Result<_, std::io::Error>>()
            .map_err(|e| LastLegendError::Io("Failed to read row buffer".into(), e))
    }
}

impl<R: Read + Seek> Iterator for RowBufferIter<R> {
    type Item = Result<Vec<u8>, LastLegendError>;

    fn next(&mut self) -> Option<Self::Item> {
        let fixed_row_size = self.fixed_row_size;
        loop {
            match &mut self.sub_row {
                SubRow::None => {
                    return self
                        .next_row_offset()
                        .map(|o| Self::default_iter(&mut self.reader, o));
                }
                SubRow::Inactive => {
                    let row_offset = self.next_row_offset()?;
                    let (data_size, row_count) = match Self::read_row_header(&mut self.reader) {
                        Ok(v) => v,
                        Err(e) => return Some(Err(e)),
                    };
                    let compute_offset = move |row_index: u64| {
                        row_offset
                            + ROW_HEADER_SIZE
                            + (row_index * fixed_row_size + 2 * (row_index + 1))
                    };
                    assert_eq!(
                        compute_offset(row_count.into()),
                        data_size.into(),
                        "Shouldn't these be equal?"
                    );
                    self.sub_row =
                        SubRow::Active(Box::new((0..u64::from(row_count)).map(compute_offset)));
                }
                SubRow::Active(iter) => {
                    let item = iter.next().map(|o| Self::default_iter(&mut self.reader, o));
                    if item.is_some() {
                        return item;
                    }
                    // No more sub-rows from this set, revert to inactive and get next set.
                    self.sub_row = SubRow::Inactive;
                }
            }
        }
    }
}
