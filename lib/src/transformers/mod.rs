use std::borrow::Cow;
use std::io::Read;

use strum::EnumString;

use crate::error::LastLegendError;
use crate::sqpath::{SqPath, SqPathBuf};
use crate::transformers::loop_ogg::LoopOgg;
use crate::transformers::scd_to_ogg::ScdToOgg;

mod loop_ogg;
mod scd_to_ogg;

pub trait Transformer<R> {
    type ForFile: TransformerForFile<R>;

    /// If this transformer applies to the given file, get a new file-specific transformer.
    fn maybe_for(&self, file: SqPathBuf) -> Option<Self::ForFile>;
}

pub trait TransformerForFile<R> {
    /// Get the file name used after the transformer is applied.
    fn renamed_file(&self) -> Cow<SqPath>;

    /// Attempt to run the transformer against the [content].
    fn transform(&self, content: R) -> Result<Box<dyn Read>, LastLegendError>;
}

#[derive(EnumString, Copy, Clone, Debug)]
#[strum(serialize_all = "snake_case")]
pub enum TransformerImpl {
    ScdToOgg,
    LoopOgg,
}

impl<R: Read> Transformer<R> for TransformerImpl {
    type ForFile = Box<dyn TransformerForFile<R>>;

    fn maybe_for(&self, file: SqPathBuf) -> Option<Self::ForFile> {
        match self {
            Self::ScdToOgg => <ScdToOgg as Transformer<R>>::maybe_for(&ScdToOgg, file)
                .map(|e| Box::new(e) as Self::ForFile),
            Self::LoopOgg => <LoopOgg as Transformer<R>>::maybe_for(&LoopOgg, file)
                .map(|e| Box::new(e) as Self::ForFile),
        }
    }
}

impl<R: Read> TransformerForFile<R> for Box<dyn TransformerForFile<R>> {
    fn renamed_file(&self) -> Cow<SqPath> {
        Box::as_ref(self).renamed_file()
    }

    fn transform(&self, content: R) -> Result<Box<dyn Read>, LastLegendError> {
        Box::as_ref(self).transform(content)
    }
}
