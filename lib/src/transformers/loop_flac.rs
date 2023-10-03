use std::borrow::Cow;
use std::io::{Cursor, Read};

use crate::error::LastLegendError;
use crate::ffmpeg::loop_using_metadata;
use crate::sqpath::{SqPath, SqPathBuf};
use crate::transformers::{Transformer, TransformerForFile};

/// Loop a `.flac` using FFMPEG.
#[derive(Debug, Default)]
pub struct LoopFlac;

impl<R: Read> Transformer<R> for LoopFlac {
    type ForFile = LoopFlacForFile;

    fn maybe_for(&self, file: SqPathBuf) -> Option<Self::ForFile> {
        file.as_str()
            .ends_with(".flac")
            .then_some(LoopFlacForFile { file })
    }
}

#[derive(Debug)]
pub struct LoopFlacForFile {
    file: SqPathBuf,
}

impl<R: Read> TransformerForFile<R> for LoopFlacForFile {
    fn renamed_file(&self) -> Cow<SqPath> {
        Cow::Borrowed(&self.file)
    }

    fn transform(&self, content: R) -> Result<Box<dyn Read>, LastLegendError> {
        let mut final_content = Vec::new();
        loop_using_metadata(content, &mut final_content)?;
        Ok(Box::new(Cursor::new(final_content)))
    }
}
