use std::borrow::Cow;
use std::io::Read;

use strum::EnumString;

use crate::error::LastLegendError;
use crate::sqpath::SqPath;
use crate::transformers::scd_to_ogg::ScdToOgg;

pub mod scd_to_ogg;

pub trait Transformer<R> {
    /// Can the given [file] be transformed?
    fn can_transform(&self, file: &SqPath) -> bool;

    /// Rename the file for after the transformer.
    /// You should only call this if [can_transform] returned `true`.
    fn rename_file(&self, file_name: &SqPath) -> Cow<SqPath>;

    /// Attempt to run the transformer against the [file]'s [content].
    /// You should only call this if [can_transform] returned `true`.
    fn transform(&self, file: &SqPath, content: R) -> Result<Box<dyn Read>, LastLegendError>;
}

#[derive(EnumString, Copy, Clone, Debug)]
#[strum(serialize_all = "snake_case")]
pub enum TransformerImpl {
    Scd,
}

impl TransformerImpl {
    pub fn into_boxed_transformer<R: Read>(self) -> Box<dyn Transformer<R>> {
        match self {
            Self::Scd => Box::new(ScdToOgg),
        }
    }
}
