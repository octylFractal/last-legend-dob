use std::borrow::Cow;
use std::io::{Cursor, Read};
use std::path::Path;

use crate::error::LastLegendError;
use crate::ffmpeg::format_rewrite;
use crate::sqpath::{SqPath, SqPathBuf};
use crate::transformers::{Transformer, TransformerForFile};

/// Change a file format using FFMPEG.
#[derive(Debug, Default)]
pub struct ChangeFile {
    pub(crate) from_extension: String,
    pub(crate) to_extension: String,
    pub(crate) to_ffmpeg_format: String,
}

impl<R: Read + Send> Transformer<R> for ChangeFile {
    type ForFile = ChangeFileForFile;

    fn maybe_for(&self, file: SqPathBuf) -> Option<Self::ForFile> {
        file.as_str()
            .ends_with(&format!(".{}", self.from_extension))
            .then_some(ChangeFileForFile {
                file,
                extension: self.to_extension.clone(),
                ffmpeg_format: self.to_ffmpeg_format.clone(),
            })
    }
}

#[derive(Debug)]
pub struct ChangeFileForFile {
    file: SqPathBuf,
    extension: String,
    ffmpeg_format: String,
}

impl<R: Read + Send> TransformerForFile<R> for ChangeFileForFile {
    fn renamed_file(&self) -> Cow<SqPath> {
        Cow::Owned(SqPathBuf::new(
            Path::new(self.file.as_str())
                .with_extension(&self.extension)
                .as_os_str()
                .to_str()
                .unwrap(),
        ))
    }

    fn transform(&self, content: R) -> Result<Box<dyn Read + Send>, LastLegendError> {
        let mut final_content = Vec::new();
        format_rewrite(&self.ffmpeg_format, content, &mut final_content)?;
        Ok(Box::new(Cursor::new(final_content)))
    }
}
