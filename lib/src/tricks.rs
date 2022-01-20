use std::fmt::{Debug, Formatter};

use binrw::{BinRead, BinWrite};

#[derive(BinRead, BinWrite)]
pub struct U32Size(
    #[br(map = |r: u32| usize::try_from(r).expect("failed to convert u32 to usize"))]
    #[bw(map = |r| u32::try_from(*r).expect("failed to convert usize to u32"))]
    pub usize,
);

impl Debug for U32Size {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
