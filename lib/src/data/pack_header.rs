use std::fmt::{Debug, Formatter};
use std::io::{Read, Seek, Write};

use binrw::{binrw, BinRead, BinResult, BinWrite, Endian};
use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc};

use crate::tricks::U32Size;

/// Gotta keep this in sync with the PackHeader below.
const HEADER_SIZE: usize =
    // for the magic
    8 +
    // for the platform id
    4 +
    // for the size itself
    4 +
    // for the version
    4 +
    // for the content type
    4 +
    // for the date
    4 +
    // for the time
    4;

#[binrw]
#[derive(Debug)]
#[brw(little, magic = b"SqPack\0\0")]
pub struct PackHeader {
    pub platform_id: PlatformId,
    pub size: U32Size,
    pub version: u32,
    pub content_type: ContentType,
    pub timestamp: SqPackTimestamp,
    // Skip the padding bytes
    #[brw(pad_before = size.0 - HEADER_SIZE)]
    padding: (),
}

#[binrw]
#[derive(Debug)]
#[brw(repr(u32))]
pub enum PlatformId {
    Win32,
    PS3,
    PS4,
}

#[binrw]
#[derive(Debug)]
#[brw(repr(u32))]
#[allow(clippy::upper_case_acronyms)]
pub enum ContentType {
    SQDB,
    Data,
    Default,
    Model,
    Image,
}

pub enum SqPackTimestamp {
    Present(DateTime<Utc>),
    Missing,
}

impl SqPackTimestamp {
    fn from_raw(date: u32, time: u32) -> Self {
        if date == 0 || time == 0 {
            return Self::Missing;
        }

        Self::Present(
            Utc.with_ymd_and_hms(
                ((date / 10000) % 10000) as i32,
                (date / 100) % 100,
                date % 100,
                (time / 1000000) % 100,
                (time / 10000) % 100,
                (time / 100) % 100,
            )
            .unwrap(),
        )
    }
}

impl Debug for SqPackTimestamp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Present(d) => d.fmt(f),
            Self::Missing => write!(f, "Missing"),
        }
    }
}

impl BinRead for SqPackTimestamp {
    type Args<'a> = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        endian: Endian,
        _: Self::Args<'_>,
    ) -> BinResult<Self> {
        let date = u32::read_options(reader, endian, ())?;
        let time = u32::read_options(reader, endian, ())?;

        Ok(Self::from_raw(date, time))
    }
}

impl BinWrite for SqPackTimestamp {
    type Args<'a> = ();

    fn write_options<W: Write + Seek>(
        &self,
        writer: &mut W,
        endian: Endian,
        _: Self::Args<'_>,
    ) -> BinResult<()> {
        let (date_u32, time_u32) = match self {
            Self::Present(d) => (
                (d.year() as u32) * 10000 + d.month() * 100 + d.day(),
                d.hour() * 1000000 + d.minute() * 10000 + d.second() * 100,
            ),
            Self::Missing => (0, 0),
        };
        date_u32.write_options(writer, endian, ())?;
        time_u32.write_options(writer, endian, ())?;
        Ok(())
    }
}
