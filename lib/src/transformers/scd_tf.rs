use std::borrow::Cow;
use std::fmt::Debug;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::Path;

use binrw::{binread, BinRead, BinReaderExt, BinResult, Endian};

use crate::error::LastLegendError;
use crate::ffmpeg::format_rewrite;
use crate::io_tricks::ReadMixer;
use crate::sqpath::{SqPath, SqPathBuf};
use crate::transformers::{Transformer, TransformerForFile};
use crate::xor::XorRead;

/// Known transformations for `.ogg` files.
#[derive(Debug, Clone, Copy)]
pub enum OggTransform {
    Ogg,
    Flac,
}

impl OggTransform {
    pub fn extension_str(&self) -> &'static str {
        match self {
            Self::Ogg => "ogg",
            Self::Flac => "flac",
        }
    }
}

/// Extract an `.ogg`-based file from the `.scd` FFXIV uses.
#[derive(Debug)]
pub struct ScdTf {
    pub(crate) ogg_transform: OggTransform,
}

impl<R: Read> Transformer<R> for ScdTf {
    type ForFile = ScdTfForFile;

    fn maybe_for(&self, file: SqPathBuf) -> Option<Self::ForFile> {
        file.as_str().ends_with(".scd").then_some(ScdTfForFile {
            file,
            ogg_transform: self.ogg_transform,
        })
    }
}

#[derive(Debug)]
pub struct ScdTfForFile {
    file: SqPathBuf,
    ogg_transform: OggTransform,
}

impl<R: Read> TransformerForFile<R> for ScdTfForFile {
    fn renamed_file(&self) -> Cow<SqPath> {
        Cow::Owned(SqPathBuf::new(
            Path::new(self.file.as_str())
                .with_extension(self.ogg_transform.extension_str())
                .as_os_str()
                .to_str()
                .unwrap(),
        ))
    }

    fn transform(&self, mut content: R) -> Result<Box<dyn Read + Send>, LastLegendError> {
        // Re-do the content as a seekable in-memory buffer.
        let content = {
            let mut capture = Vec::<u8>::new();
            content
                .read_to_end(&mut capture)
                .map_err(|e| LastLegendError::Io("Couldn't cache content".into(), e))?;
            drop(content);
            Cursor::new(capture)
        };
        self.decode(content)
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

impl ScdTfForFile {
    fn decode(
        &self,
        mut content: Cursor<Vec<u8>>,
    ) -> Result<Box<dyn Read + Send>, LastLegendError> {
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
        let mut ogg_reader =
            if scd.ogg_seek_header.encryption_type == EncryptionType::InternalTableXor {
                let static_xor = (scd.sound_entry_header.data_size & 0x7F) as u8;
                let table_off = (scd.sound_entry_header.data_size & 0x3F) as u8;
                ReadMixer::Wrapped(XorRead::new(base, move |index| {
                    XOR_TABLE[(usize::from(table_off) + index) & 0xFF] ^ static_xor
                }))
            } else {
                ReadMixer::Plain(base)
            };
        match self.ogg_transform {
            OggTransform::Ogg => Ok(Box::new(ogg_reader)),
            OggTransform::Flac => {
                let mut final_content = Vec::new();
                format_rewrite("flac", &mut ogg_reader, &mut final_content)?;
                Ok(Box::new(Cursor::new(final_content)))
            }
        }
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
    _aux_chunk_devnull: AuxChunkDevNull,
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
    _channels: u32,
    #[br(temp)]
    _frequency: u32,
    pub data_type: DataType,
    #[br(temp)]
    _loop_start: u32,
    #[br(temp)]
    _loop_end: u32,
    #[br(temp)]
    _first_frame_pos: u32,
    #[br(pad_after = 2)]
    pub aux_chunk_count: u16,
}

#[binread]
#[derive(Debug, Eq, PartialEq)]
#[br(repr(i32))]
enum DataType {
    Empty = -1,
    Ogg = 0x6,
    MsAdpcm = 0xC,
}

/// An adapter that jumps to the end of the aux chunk table.
struct AuxChunkDevNull;

impl BinRead for AuxChunkDevNull {
    type Args<'a> = (u16,);

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _: Endian,
        (count,): Self::Args<'_>,
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
    _seek_table: Vec<u32>,
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
