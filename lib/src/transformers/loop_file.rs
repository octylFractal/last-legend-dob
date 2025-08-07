use std::borrow::Cow;
use std::io::{Cursor, Read};

use crate::error::LastLegendError;
use crate::ffmpeg::loop_using_metadata;
use crate::sqpath::{SqPath, SqPathBuf};
use crate::transformers::{Transformer, TransformerForFile};

/// Loop a file using FFMPEG.
#[derive(Debug, Default)]
pub struct LoopFile {
    pub(crate) extension: String,
    pub(crate) ffmpeg_format: String,
}

impl<R: Read> Transformer<R> for LoopFile {
    type ForFile = LoopFileForFile;

    fn maybe_for(&self, file: SqPathBuf) -> Option<Self::ForFile> {
        file.as_str()
            .ends_with(&format!(".{}", self.extension))
            .then_some(LoopFileForFile {
                file,
                ffmpeg_format: self.ffmpeg_format.clone(),
            })
    }
}

#[derive(Debug)]
pub struct LoopFileForFile {
    file: SqPathBuf,
    ffmpeg_format: String,
}

impl<R: Read> TransformerForFile<R> for LoopFileForFile {
    fn renamed_file(&'_ self) -> Cow<'_, SqPath> {
        Cow::Borrowed(&self.file)
    }

    fn transform(&self, content: R) -> Result<Box<dyn Read + Send>, LastLegendError> {
        let mut final_content = Vec::new();
        loop_using_metadata(&self.ffmpeg_format, content, &mut final_content)?;
        Ok(Box::new(Cursor::new(final_content)))
    }
}
