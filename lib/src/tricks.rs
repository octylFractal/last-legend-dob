use std::ffi::OsString;
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

#[derive(Clone, Debug)]
pub struct ArgBuilder {
    parts: Vec<OsString>,
}

impl ArgBuilder {
    pub fn new() -> Self {
        Self { parts: Vec::new() }
    }

    pub fn add(mut self, part: impl Into<OsString>) -> Self {
        self.parts.push(part.into());
        self
    }

    pub fn add_kv(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.parts.extend_from_slice(&[key.into(), value.into()]);
        self
    }

    pub fn add_all(mut self, part: impl IntoIterator<Item = impl Into<OsString>>) -> Self {
        self.parts.extend(part.into_iter().map(Into::into));
        self
    }

    pub fn into_vec(self) -> Vec<OsString> {
        self.parts
    }
}
