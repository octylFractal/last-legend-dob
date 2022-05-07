use std::io::{Read, Seek, SeekFrom};

use binrw::{binread, binrw, BinReaderExt};
use flate2::read::DeflateDecoder;

use crate::io_tricks::ReadMixer;

// I didn't write a Dat reader, since that's not really needed.
/// Dat Entry Header reader, find entries using the [Index2].
#[binread]
#[derive(Debug)]
#[br(little)]
pub struct DatEntryHeader {
    header_size: u32,
    #[br(temp)]
    content_type: ContentType,
    pub uncompressed_size: u32,
    #[br(temp)]
    unknown: u32,
    pub block_size: u32,
    pub num_blocks: u32,
    #[br(args { content_type, num_blocks })]
    blocks: DatEntryHeaderBlocks,
}

impl DatEntryHeader {
    /// Given a [reader], positioned at the start of the header, get a new reader for the content.
    pub fn read_content<R: Read + Seek>(
        &self,
        mut reader: R,
    ) -> std::io::Result<DatEntryContent<R>> {
        let DatEntryHeaderBlocks::Binary(blocks) = &self.blocks;
        let stream_pos = reader.stream_position()?;
        Ok(DatEntryContent {
            inner: reader,
            base_pos: stream_pos + u64::from(self.header_size),
            block_iter: blocks.iter(),
            buf: None,
        })
    }

    /// Given a [reader], positioned at the start of the header, read the content to a [Vec].
    pub fn read_content_to_vec<R: Read + Seek>(&self, reader: R) -> std::io::Result<Vec<u8>> {
        let mut content = Vec::with_capacity(self.uncompressed_size.try_into().unwrap());
        self.read_content(reader)?.read_to_end(&mut content)?;
        assert_eq!(
            usize::try_from(self.uncompressed_size).unwrap(),
            content.len()
        );

        Ok(content)
    }
}

pub struct DatEntryContent<'a, R> {
    inner: R,
    /// Starting position for computing relative offsets.
    base_pos: u64,
    /// The iterator over the blocks.
    block_iter: std::slice::Iter<'a, BinaryDatEntryHeaderBlock>,
    /// The buffer for the last read content block.
    buf: Option<Buffer>,
}

impl<'a, R: Read + Seek> DatEntryContent<'a, R> {
    /// Finish using the content reader, and get back the original reader.
    /// The position will not be adjusted.
    pub fn into_inner(self) -> R {
        self.inner
    }

    fn read_block(&mut self, block: &BinaryDatEntryHeaderBlock) -> std::io::Result<()> {
        self.inner
            .seek(SeekFrom::Start(self.base_pos + u64::from(block.offset)))?;
        let header: DataBlockHeader = self
            .inner
            .read_le()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        assert_eq!(
            header.decompressed_size(),
            block.decompressed_size.into(),
            "Block headers disagree on decompressed size!"
        );
        let base_reader = (&mut self.inner).take(header.source_size().into());
        let mut reader = if header.is_compressed() {
            ReadMixer::Wrapped(DeflateDecoder::new(base_reader))
        } else {
            ReadMixer::Plain(base_reader)
        };

        let buffer = self.buf.as_mut().unwrap();
        let limit = header.decompressed_size() as usize;
        reader.read_exact(&mut buffer.content[0..limit])?;
        buffer.pos = 0;
        buffer.limit = limit;

        Ok(())
    }
}

impl<'a, R: Read + Seek> Read for DatEntryContent<'a, R> {
    fn read(&mut self, output_buf: &mut [u8]) -> std::io::Result<usize> {
        let mut buf = match &mut self.buf {
            Some(buf) if buf.can_read() => buf,
            _ => {
                let next_block = match self.block_iter.next() {
                    Some(b) => b,
                    None => {
                        // free the buf in advance
                        self.buf = None;
                        return Ok(0);
                    }
                };
                // Check if we need a buffer, which includes if the current buffer is too small.
                if self.buf.is_none()
                    || matches!(&self.buf, Some(b) if b.content.len() < next_block.decompressed_size.into())
                {
                    self.buf = Some(Buffer::with_capacity(next_block.decompressed_size.into()));
                }
                // Fill the buffer with the next block
                self.read_block(next_block)?;

                self.buf.as_mut().unwrap()
            }
        };

        let len = buf.len().min(output_buf.len());
        (output_buf[..len]).copy_from_slice(&buf.content[buf.pos..(buf.pos + len)]);
        buf.pos += len;
        Ok(len)
    }
}

// TODO: Implement Seek?

struct Buffer {
    pub content: Box<[u8]>,
    pub pos: usize,
    pub limit: usize,
}

impl Buffer {
    pub fn with_capacity(capacity: u32) -> Self {
        Self {
            content: vec![0u8; capacity.try_into().unwrap()].into_boxed_slice(),
            pos: 0,
            limit: 0,
        }
    }

    pub fn can_read(&self) -> bool {
        self.len() > 0
    }

    pub fn len(&self) -> usize {
        self.limit - self.pos
    }
}

#[binread]
#[derive(Debug)]
#[br(import { content_type: ContentType, num_blocks: u32 })]
pub enum DatEntryHeaderBlocks {
    #[br(pre_assert(content_type == ContentType::Binary))]
    Binary(#[br(args { count: num_blocks.try_into().unwrap() })] Vec<BinaryDatEntryHeaderBlock>),
}

impl DatEntryHeaderBlocks {
    pub fn content_type(&self) -> ContentType {
        match self {
            Self::Binary(..) => ContentType::Binary,
        }
    }
}

#[binread]
#[derive(Debug)]
pub struct BinaryDatEntryHeaderBlock {
    pub offset: u32,
    pub block_size: u16,
    pub decompressed_size: u16,
}

const KNOWN_HEADER_SIZE: u32 = 0x10;

#[binread]
#[derive(Debug)]
struct DataBlockHeader {
    #[br(temp, assert(header_size == KNOWN_HEADER_SIZE))]
    header_size: u32,
    #[br(pad_before = 0x4)]
    compressed_length: u32,
    decompressed_length: u32,
}

impl DataBlockHeader {
    pub fn is_compressed(&self) -> bool {
        const NOT_COMPRESSED: u32 = 32_000;
        if self.compressed_length < NOT_COMPRESSED {
            return true;
        }
        assert_eq!(self.compressed_length, NOT_COMPRESSED);
        false
    }

    pub fn source_size(&self) -> u32 {
        if self.is_compressed() {
            // Refer to https://github.com/xivapi/SaintCoinach/blob/f2af100a7d4225f04c2f534bbbc63caf60719766/SaintCoinach/IO/File.cs#L103-L109
            const BLOCK_PADDING: u32 = 0x80;
            let padding_check = (self.compressed_length + KNOWN_HEADER_SIZE) % BLOCK_PADDING;
            if padding_check != 0 {
                self.compressed_length + (BLOCK_PADDING - padding_check)
            } else {
                self.compressed_length
            }
        } else {
            self.decompressed_length
        }
    }

    pub fn decompressed_size(&self) -> u32 {
        self.decompressed_length
    }
}

#[binrw]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
#[brw(repr(u32))]
pub enum ContentType {
    Empty = 1,
    Binary,
    Model,
    Texture,
}
