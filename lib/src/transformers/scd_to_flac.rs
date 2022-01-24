use std::borrow::Cow;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::sync::{Arc, Mutex};

use binrw::{binread, BinRead, BinReaderExt, BinResult, ReadOptions};

use crate::error::LastLegendError;
use crate::ffmpeg::loop_using_metadata;
use crate::io_tricks::{ArcWrite, ReadMixer};
use crate::sqpath::{SqPath, SqPathBuf};
use crate::transformers::Transformer;
use crate::xor::XorRead;

/// Extract a `.flac` from the `.scd` FFXIV uses.
///
/// Notes:
/// - Only extracts one file at a time (obviously).
/// - Applies the loop using FFMPEG.
#[derive(Debug, Default)]
pub struct ScdToFlac;

impl<R: Read> Transformer<R> for ScdToFlac {
    fn can_transform(&self, file: &SqPath) -> bool {
        file.as_str().ends_with(".scd")
    }

    fn rename_file(&self, file_name: &SqPath) -> Cow<SqPath> {
        Cow::Owned(SqPathBuf::new(&file_name.as_str().replace(".scd", ".flac")))
    }

    fn transform(&self, _: &SqPath, mut content: R) -> Result<Box<dyn Read>, LastLegendError> {
        // Re-do the content as a seekable in-memory buffer.
        let content = {
            let mut capture = Vec::<u8>::new();
            content
                .read_to_end(&mut capture)
                .map_err(|e| LastLegendError::Io("Couldn't cache content".into(), e))?;
            drop(content);
            Cursor::new(capture)
        };
        Self::decode(content)
    }
}

const XOR_TABLE: &[u8; 256] = &[
    0x3A, 0x32, 0x32, 0x32, 0x03, 0x7E, 0x12, 0xF7, 0xB2, 0xE2, 0xA2, 0x67, 0x32, 0x32, 0x22, 0x32,
    0x32, 0x52, 0x16, 0x1B, 0x3C, 0xA1, 0x54, 0x7B, 0x1B, 0x97, 0xA6, 0x93, 0x1A, 0x4B, 0xAA, 0xA6,
    0x7A, 0x7B, 0x1B, 0x97, 0xA6, 0xF7, 0x02, 0xBB, 0xAA, 0xA6, 0xBB, 0xF7, 0x2A, 0x51, 0xBE, 0x03,
    0xF4, 0x2A, 0x51, 0xBE, 0x03, 0xF4, 0x2A, 0x51, 0xBE, 0x12, 0x06, 0x56, 0x27, 0x32, 0x32, 0x36,
    0x32, 0xB2, 0x1A, 0x3B, 0xBC, 0x91, 0xD4, 0x7B, 0x58, 0xFC, 0x0B, 0x55, 0x2A, 0x15, 0xBC, 0x40,
    0x92, 0x0B, 0x5B, 0x7C, 0x0A, 0x95, 0x12, 0x35, 0xB8, 0x63, 0xD2, 0x0B, 0x3B, 0xF0, 0xC7, 0x14,
    0x51, 0x5C, 0x94, 0x86, 0x94, 0x59, 0x5C, 0xFC, 0x1B, 0x17, 0x3A, 0x3F, 0x6B, 0x37, 0x32, 0x32,
    0x30, 0x32, 0x72, 0x7A, 0x13, 0xB7, 0x26, 0x60, 0x7A, 0x13, 0xB7, 0x26, 0x50, 0xBA, 0x13, 0xB4,
    0x2A, 0x50, 0xBA, 0x13, 0xB5, 0x2E, 0x40, 0xFA, 0x13, 0x95, 0xAE, 0x40, 0x38, 0x18, 0x9A, 0x92,
    0xB0, 0x38, 0x00, 0xFA, 0x12, 0xB1, 0x7E, 0x00, 0xDB, 0x96, 0xA1, 0x7C, 0x08, 0xDB, 0x9A, 0x91,
    0xBC, 0x08, 0xD8, 0x1A, 0x86, 0xE2, 0x70, 0x39, 0x1F, 0x86, 0xE0, 0x78, 0x7E, 0x03, 0xE7, 0x64,
    0x51, 0x9C, 0x8F, 0x34, 0x6F, 0x4E, 0x41, 0xFC, 0x0B, 0xD5, 0xAE, 0x41, 0xFC, 0x0B, 0xD5, 0xAE,
    0x41, 0xFC, 0x3B, 0x70, 0x71, 0x64, 0x33, 0x32, 0x12, 0x32, 0x32, 0x36, 0x70, 0x34, 0x2B, 0x56,
    0x22, 0x70, 0x3A, 0x13, 0xB7, 0x26, 0x60, 0xBA, 0x1B, 0x94, 0xAA, 0x40, 0x38, 0x00, 0xFA, 0xB2,
    0xE2, 0xA2, 0x67, 0x32, 0x32, 0x12, 0x32, 0xB2, 0x32, 0x32, 0x32, 0x32, 0x75, 0xA3, 0x26, 0x7B,
    0x83, 0x26, 0xF9, 0x83, 0x2E, 0xFF, 0xE3, 0x16, 0x7D, 0xC0, 0x1E, 0x63, 0x21, 0x07, 0xE3, 0x01,
];

impl ScdToFlac {
    fn decode(mut content: Cursor<Vec<u8>>) -> Result<Box<dyn Read>, LastLegendError> {
        let scd: Scd = content
            .read_le()
            .map_err(|e| LastLegendError::BinRW("Couldn't read SCD".into(), e))?;
        let vorbis_header =
            if scd.ogg_seek_header.encryption_type == EncryptionType::VorbisHeaderXor {
                ReadMixer::Wrapped(XorRead::new(
                    Cursor::new(scd.ogg_seek_header.vorbis_header),
                    move |_| scd.ogg_seek_header.xor_byte,
                ))
            } else {
                ReadMixer::Plain(Cursor::new(scd.ogg_seek_header.vorbis_header))
            };
        let base = vorbis_header.chain(content.take(scd.sound_entry_header.data_size.into()));
        let ogg_reader = if scd.ogg_seek_header.encryption_type == EncryptionType::InternalTableXor
        {
            let static_xor = (scd.sound_entry_header.data_size & 0x7F) as u8;
            let table_off = (scd.sound_entry_header.data_size & 0x3F) as u8;
            ReadMixer::Wrapped(XorRead::new(base, move |index| {
                XOR_TABLE[(usize::from(table_off) + index) & 0xFF] ^ static_xor
            }))
        } else {
            ReadMixer::Plain(base)
        };
        let final_content = Arc::new(Mutex::new(Vec::new()));
        loop_using_metadata(ogg_reader, ArcWrite::new(Arc::clone(&final_content)))?;
        Ok(Box::new(Cursor::new(
            Arc::try_unwrap(final_content)
                .unwrap()
                .into_inner()
                .expect("lock poisoned"),
        )))
    }
}

#[binread]
#[derive(Debug)]
#[br(magic = b"SEDBSSCF")]
struct Scd {
    #[br(temp, assert(version == 3))]
    version: u32,
    #[br(temp, pad_before = 2)]
    header_size: u16,
    #[br(
        temp,
        seek_before = SeekFrom::Start(header_size.into()),
        assert(offsets_header.sound_entries_size == 1, "Only one entry is supported currently.")
    )]
    offsets_header: ScdOffsetsHeader,
    #[br(temp, seek_before = SeekFrom::Start(offsets_header.sound_entries_offset.into()))]
    entry_table_offset: u32,
    #[br(
        seek_before = SeekFrom::Start(entry_table_offset.into()),
        assert(sound_entry_header.data_type == DataType::Ogg, "Only OGG supported"),
    )]
    pub sound_entry_header: SoundEntryHeader,
    #[br(temp, args(sound_entry_header.aux_chunk_count))]
    aux_chunk_devnull: AuxChunkDevNull,
    pub ogg_seek_header: OggMetaHeader,
}

#[binread]
#[derive(Debug)]
struct ScdOffsetsHeader {
    #[br(pad_before = 4)]
    pub sound_entries_size: u16,
    #[br(pad_before = 0x6)]
    pub sound_entries_offset: u32,
}

#[binread]
#[derive(Debug)]
struct SoundEntryHeader {
    pub data_size: u32,
    #[br(temp)]
    channels: u32,
    #[br(temp)]
    frequency: u32,
    pub data_type: DataType,
    #[br(temp)]
    loop_start: u32,
    #[br(temp)]
    loop_end: u32,
    #[br(temp)]
    first_frame_pos: u32,
    #[br(pad_after = 2)]
    pub aux_chunk_count: u16,
}

#[binread]
#[derive(Debug, Eq, PartialEq)]
#[br(repr(u32))]
enum DataType {
    Ogg = 0x6,
    MsAdpcm = 0xC,
}

/// An adapter that jumps to the end of the aux chunk table.
struct AuxChunkDevNull;

impl BinRead for AuxChunkDevNull {
    type Args = (u16,);

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _: &ReadOptions,
        (count,): Self::Args,
    ) -> BinResult<Self> {
        let mut position = reader.stream_position()?;
        for _ in 0..count {
            reader.seek(SeekFrom::Start(position + 4))?;
            position += u64::from(reader.read_le::<u32>()?);
        }
        reader.seek(SeekFrom::Start(position))?;
        Ok(AuxChunkDevNull)
    }
}

#[binread]
#[derive(Debug)]
struct OggMetaHeader {
    pub encryption_type: EncryptionType,
    pub xor_byte: u8,
    #[br(temp, pad_before = 0xD)]
    seek_table_size: u32,
    #[br(temp, pad_after = 0x8)]
    vorbis_header_size: u32,
    #[br(temp, args { count: usize::try_from(seek_table_size).unwrap() / 4 })]
    seek_table: Vec<u32>,
    /// May be encoded. Decoding is done separately.
    #[br(args { count: vorbis_header_size.try_into().unwrap() })]
    pub vorbis_header: Vec<u8>,
}

#[binread]
#[derive(Debug, Eq, PartialEq)]
#[br(repr(u16))]
enum EncryptionType {
    None,
    VorbisHeaderXor = 0x2002,
    InternalTableXor = 0x2003,
}
