use crate::tricks::U32Size;
use binrw::binread;

/// Gotta keep this in sync with the IndexHeader below.
const HEADER_SIZE: usize =
    // for the size itself
    4 +
    // for the index type
    4 +
    // for the data offset
    4 +
    // for the data size
    4;

#[binread]
#[derive(Debug)]
#[brw(little)]
pub struct IndexHeader {
    pub size: U32Size,
    // This appears to always be 1.
    #[br(assert(index_type == 1))]
    pub index_type: u32,
    pub index_data_offset: u32,
    pub index_data_size: U32Size,
    // Skip the padding bytes
    #[brw(temp, pad_before = size.0 - HEADER_SIZE)]
    _padding: (),
}
