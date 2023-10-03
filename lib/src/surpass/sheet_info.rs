use std::io::{Read, Seek, SeekFrom};
use std::ops::Range;

use binrw::helpers::count_with;
use binrw::{binread, BinRead, BinReaderExt, BinResult, NullString};

use crate::error::LastLegendError;

#[binread]
#[derive(Debug, Clone)]
#[br(big, magic = b"EXHF")]
pub struct SheetInfo {
    #[br(temp)]
    _unknown_1: [u8; 2],
    pub fixed_row_size: u16,
    #[br(temp)]
    column_count: u16,
    #[br(temp)]
    page_count: u16,
    #[br(temp)]
    language_count: u16,
    #[br(temp)]
    _unknown_3: [u8; 2],
    pub variant: Variant,
    #[br(temp)]
    _unknown_4: [u8; 14],
    #[br(args { count: dbg!(column_count).try_into().unwrap() })]
    pub columns: Vec<Column>,
    #[br(parse_with = count_with(
        dbg!(page_count).try_into().unwrap(),
        range_parser
    ))]
    pub page_ranges: Vec<Range<u32>>,
    #[br(args { count: dbg!(language_count).try_into().unwrap() })]
    pub languages: Vec<Language>,
}

#[binread]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[br(repr(u16))]
pub enum Variant {
    Default = 1,
    SubRows = 2,
}

#[binread]
#[derive(Debug, Copy, Clone)]
pub struct Column {
    data_type: DataType,
    offset: u16,
}

impl Column {
    pub fn read_value<R: Read + Seek>(
        &self,
        mut reader: R,
        fixed_row_size: u64,
    ) -> Result<DataValue, LastLegendError> {
        reader
            .seek(SeekFrom::Start(u64::from(self.offset)))
            .map_err(|e| LastLegendError::Io("Failed to move to data pos".into(), e))?;
        match self.data_type {
            DataType::String => {
                let str_offset =
                    u64::from(reader.read_be::<u32>().map_err(|e| {
                        LastLegendError::BinRW("Failed to read str offset".into(), e)
                    })?);
                reader
                    .seek(SeekFrom::Start(fixed_row_size + str_offset))
                    .map_err(|e| LastLegendError::Io("Failed to move to str pos".into(), e))?;
                let nstr = reader
                    .read_be::<NullString>()
                    .map_err(|e| LastLegendError::BinRW("Failed to read str".into(), e))?;
                Ok(DataValue::String(
                    nstr.try_into().expect("Failed to convert string"),
                ))
            }
            DataType::Bool => reader
                .read_be::<u8>()
                .map_err(|e| LastLegendError::BinRW("Failed to read bool".into(), e))
                .map(|b| DataValue::Bool(b == 1)),
            DataType::I8 => reader
                .read_be::<i8>()
                .map_err(|e| LastLegendError::BinRW("Failed to read i8".into(), e))
                .map(DataValue::I8),
            DataType::U8 => reader
                .read_be::<u8>()
                .map_err(|e| LastLegendError::BinRW("Failed to read u8".into(), e))
                .map(DataValue::U8),
            DataType::I16 => reader
                .read_be::<i16>()
                .map_err(|e| LastLegendError::BinRW("Failed to read i16".into(), e))
                .map(DataValue::I16),
            DataType::U16 => reader
                .read_be::<u16>()
                .map_err(|e| LastLegendError::BinRW("Failed to read u16".into(), e))
                .map(DataValue::U16),
            DataType::I32 => reader
                .read_be::<i32>()
                .map_err(|e| LastLegendError::BinRW("Failed to read i32".into(), e))
                .map(DataValue::I32),
            DataType::U32 => reader
                .read_be::<u32>()
                .map_err(|e| LastLegendError::BinRW("Failed to read u32".into(), e))
                .map(DataValue::U32),
            DataType::F32 => reader
                .read_be::<f32>()
                .map_err(|e| LastLegendError::BinRW("Failed to read f32".into(), e))
                .map(DataValue::F32),
            DataType::I64 => reader
                .read_be::<i64>()
                .map_err(|e| LastLegendError::BinRW("Failed to read i64".into(), e))
                .map(DataValue::I64),
            DataType::PackedBool0
            | DataType::PackedBool1
            | DataType::PackedBool2
            | DataType::PackedBool3
            | DataType::PackedBool4
            | DataType::PackedBool5
            | DataType::PackedBool6
            | DataType::PackedBool7 => reader
                .read_be::<u8>()
                .map_err(|e| LastLegendError::BinRW("Failed to read packed bool".into(), e))
                .map(|b| {
                    let bit = 1 >> (self.data_type as u8 - DataType::PackedBool0 as u8);
                    DataValue::Bool((b & bit) == bit)
                }),
        }
    }
}

#[binread]
#[derive(Debug, Copy, Clone)]
#[br(repr(u16))]
pub enum DataType {
    String,
    Bool,
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    F32 = 0x9,
    I64 = 0xB,
    PackedBool0 = 0x19,
    PackedBool1,
    PackedBool2,
    PackedBool3,
    PackedBool4,
    PackedBool5,
    PackedBool6,
    PackedBool7,
}

#[derive(Debug, Clone)]
pub enum DataValue {
    String(String),
    Bool(bool),
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    F32(f32),
    I64(i64),
    // Packed bools are Bool
}

#[binrw::parser(reader, endian)]
fn range_parser(_: ()) -> BinResult<Range<u32>> {
    #[binread]
    #[derive(Debug)]
    struct FileRange {
        min: u32,
        len: u32,
    }

    let res: FileRange = FileRange::read_options(reader, endian, ())?;
    Ok(Range {
        start: res.min,
        end: res.min + res.len,
    })
}

#[binread]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
#[br(little, repr(u16))]
pub enum Language {
    None,
    Japanese,
    English,
    German,
    French,
    ChineseSimplified,
    ChineseTraditional,
    Korean,
}

impl Language {
    pub fn get_sheet_name(&self, sheet_name: &str, start_id: u32) -> String {
        let lang_code = match self {
            Language::None => {
                return format!("exd/{}_{}.exd", sheet_name, start_id);
            }
            Language::Japanese => "ja",
            Language::English => "en",
            Language::German => "de",
            Language::French => "fr",
            Language::ChineseSimplified => "chs",
            Language::ChineseTraditional => "cht",
            Language::Korean => "ko",
        };
        format!("exd/{}_{}_{}.exd", sheet_name, start_id, lang_code)
    }
}
