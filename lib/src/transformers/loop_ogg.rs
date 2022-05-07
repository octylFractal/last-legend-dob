use std::borrow::Cow;
use std::io::{Cursor, Read};

use crate::error::LastLegendError;
use crate::ffmpeg::loop_using_metadata;
use crate::sqpath::{SqPath, SqPathBuf};
use crate::transformers::{Transformer, TransformerForFile};

/// Loop a `.ogg` using FFMPEG.
#[derive(Debug, Default)]
pub struct LoopOgg;

impl<R: Read> Transformer<R> for LoopOgg {
    type ForFile = LoopOggForFile;

    fn maybe_for(&self, file: SqPathBuf) -> Option<Self::ForFile> {
        file.as_str()
            .ends_with(".ogg")
            .then(|| LoopOggForFile { file })
    }
}

#[derive(Debug)]
pub struct LoopOggForFile {
    file: SqPathBuf,
}

impl<R: Read> TransformerForFile<R> for LoopOggForFile {
    fn renamed_file(&self) -> Cow<SqPath> {
        Cow::Borrowed(&self.file)
    }

    fn transform(&self, content: R) -> Result<Box<dyn Read>, LastLegendError> {
        let mut final_content = Vec::new();
        loop_using_metadata(content, &mut final_content)?;
        Ok(Box::new(Cursor::new(final_content)))
    }
}
