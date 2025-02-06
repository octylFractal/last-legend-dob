use std::borrow::Cow;
use std::io::Read;

use strum::EnumString;

use crate::error::LastLegendError;
use crate::sqpath::{SqPath, SqPathBuf};
use crate::transformers::change_format::ChangeFile;
use crate::transformers::loop_file::LoopFile;
use crate::transformers::scd_tf::{ScdAudioTransform, ScdTf};

mod change_format;
mod loop_file;
mod scd_tf;

pub trait Transformer<R> {
    type ForFile: TransformerForFile<R>;

    /// If this transformer applies to the given file, get a new file-specific transformer.
    fn maybe_for(&self, file: SqPathBuf) -> Option<Self::ForFile>;
}

pub trait TransformerForFile<R> {
    /// Get the file name used after the transformer is applied.
    fn renamed_file(&self) -> Cow<SqPath>;

    /// Attempt to run the transformer against the [content].
    fn transform(&self, content: R) -> Result<Box<dyn Read + Send>, LastLegendError>;
}

#[derive(EnumString, Copy, Clone, Debug)]
#[strum(serialize_all = "snake_case")]
pub enum TransformerImpl {
    ScdToFlac,
    LoopFlac,
    ScdToOgg,
    LoopOgg,
    FlacToOgg,
    ScdToWav,
}

impl<R: Read + Send> Transformer<R> for TransformerImpl {
    type ForFile = Box<dyn TransformerForFile<R>>;

    fn maybe_for(&self, file: SqPathBuf) -> Option<Self::ForFile> {
        match self {
            Self::ScdToFlac => <ScdTf as Transformer<R>>::maybe_for(
                &ScdTf {
                    audio_transform: ScdAudioTransform::Flac,
                },
                file,
            )
            .map(|e| Box::new(e) as Self::ForFile),
            Self::LoopFlac => <LoopFile as Transformer<R>>::maybe_for(
                &LoopFile {
                    extension: "flac".to_string(),
                    ffmpeg_format: "flac".to_string(),
                },
                file,
            )
            .map(|e| Box::new(e) as Self::ForFile),
            Self::ScdToOgg => <ScdTf as Transformer<R>>::maybe_for(
                &ScdTf {
                    audio_transform: ScdAudioTransform::Ogg,
                },
                file,
            )
            .map(|e| Box::new(e) as Self::ForFile),
            Self::LoopOgg => <LoopFile as Transformer<R>>::maybe_for(
                &LoopFile {
                    extension: "ogg".to_string(),
                    ffmpeg_format: "ogg".to_string(),
                },
                file,
            )
            .map(|e| Box::new(e) as Self::ForFile),
            Self::FlacToOgg => <ChangeFile as Transformer<R>>::maybe_for(
                &ChangeFile {
                    from_extension: "flac".to_string(),
                    to_extension: "ogg".to_string(),
                    to_ffmpeg_format: "ogg".to_string(),
                },
                file,
            )
            .map(|e| Box::new(e) as Self::ForFile),
            Self::ScdToWav => <ScdTf as Transformer<R>>::maybe_for(
                &ScdTf {
                    audio_transform: ScdAudioTransform::Wav,
                },
                file,
            )
            .map(|e| Box::new(e) as Self::ForFile),
        }
    }
}

impl<R: Read> TransformerForFile<R> for Box<dyn TransformerForFile<R>> {
    fn renamed_file(&self) -> Cow<SqPath> {
        Box::as_ref(self).renamed_file()
    }

    fn transform(&self, content: R) -> Result<Box<dyn Read + Send>, LastLegendError> {
        Box::as_ref(self).transform(content)
    }
}
