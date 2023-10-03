use std::fs::File;
use std::io::{ErrorKind, Read, Write};
use std::ops::{Deref, DerefMut};
use std::process::{Child, Command, Output, Stdio};

use crate::error::LastLegendError;
use crate::tricks::ArgBuilder;

const GENERAL_FFMPEG_INSTRUCTIONS: [&str; 1] = ["-hide_banner"];

/// Loop a FLAC using the Loopstart and Loopend metadata.
pub fn loop_using_metadata(
    mut reader: impl Read,
    mut output: impl Write,
) -> Result<(), LastLegendError> {
    let mut original_cache_file = tempfile::NamedTempFile::new()
        .map_err(|e| LastLegendError::Io("Couldn't create temporary cache file".into(), e))?;
    let looped_cache_file = tempfile::NamedTempFile::new()
        .map_err(|e| LastLegendError::Io("Couldn't create temporary loop cache file".into(), e))?;
    // dump the reader to a file for probing
    std::io::copy(&mut reader, original_cache_file.as_file_mut())
        .map_err(|e| LastLegendError::Io("Couldn't copy to original cache file".into(), e))?;

    // Run FFMPEG command to tell me what the loop points are
    let probe_args = ArgBuilder::new()
        .add_all(GENERAL_FFMPEG_INSTRUCTIONS)
        .add_all(get_ffmpeg_loglevel())
        .add_kv("-i", original_cache_file.path())
        .add_kv("-show_entries", "format_tags")
        .add_kv("-of", "compact=p=0:nk=1")
        .into_vec();
    log::debug!("Running ffprobe {:?}", probe_args);
    let audio_probe_output = Command::new("ffprobe")
        .args(probe_args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .output()
        .map_err(|e| LastLegendError::Io("Couldn't run ffprobe".into(), e))?;
    check_exit(&audio_probe_output)?;
    let (loop_start, loop_end): (u32, u32) = {
        let stdout = String::from_utf8_lossy(&audio_probe_output.stdout).into_owned();
        let output = stdout
            .lines()
            .next()
            .map(|line| line.split('|').collect::<Vec<_>>())
            .ok_or_else(|| LastLegendError::FFMPEG("no output".to_string()))?;
        match output.as_slice() {
            &[loop_start, loop_end, ..] => {
                let loop_start = loop_start.parse().map_err(|_| {
                    LastLegendError::FFMPEG(format!(
                        "audio loop_start wasn't a u32 but: {}",
                        loop_start
                    ))
                })?;
                let loop_end = loop_end.parse().map_err(|_| {
                    LastLegendError::FFMPEG(format!(
                        "audio duration wasn't a u32 but: {}",
                        loop_end
                    ))
                })?;
                (loop_start, loop_end)
            }
            _ => (0, 0),
        }
    };

    // Run FFMPEG command to loop the audio (if the loop point isn't just 0)
    match loop_start {
        0 => {
            // N.B. do not check loop_end here, it is 0 sometimes.
            // We can just do an in-process file copy
            std::io::copy(
                &mut File::open(original_cache_file.path()).map_err(|e| {
                    LastLegendError::Io("Couldn't open original cache file".into(), e)
                })?,
                &mut File::create(looped_cache_file.path()).map_err(|e| {
                    LastLegendError::Io("Couldn't open looped cache file".into(), e)
                })?,
            )
            .map_err(|e| {
                LastLegendError::Io("Couldn't copy original file to looped file".into(), e)
            })?;
        }
        _ => {
            let ffmpeg_args = ArgBuilder::new()
                .add_all(GENERAL_FFMPEG_INSTRUCTIONS)
                .add_all(get_ffmpeg_loglevel())
                .add("-y")
                .add_kv("-i", original_cache_file.path())
                .add_kv(
                    "-af",
                    format!(
                        "aloop=loop=1:start={}:size={}",
                        loop_start,
                        loop_end - loop_start
                    ),
                )
                .add_kv("-f", "flac")
                .add(looped_cache_file.path())
                .into_vec();
            log::debug!("Running ffmpeg {:?}", ffmpeg_args);
            let ffmpeg_loop_output = Command::new("ffmpeg")
                .args(ffmpeg_args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .output()
                .map_err(|e| LastLegendError::Io("Couldn't run ffmpeg".into(), e))?;
            check_exit(&ffmpeg_loop_output)?;
        }
    }

    // Run FFMPEG command to tell me what the length is
    let probe_args = ArgBuilder::new()
        .add_all(GENERAL_FFMPEG_INSTRUCTIONS)
        .add_all(get_ffmpeg_loglevel())
        .add_kv("-i", looped_cache_file.path())
        .add_kv("-show_entries", "stream=duration")
        .add_kv("-of", "compact=p=0:nk=1")
        .into_vec();
    log::debug!("Running ffprobe {:?}", probe_args);
    let audio_probe_output = Command::new("ffprobe")
        .args(probe_args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .output()
        .map_err(|e| LastLegendError::Io("Couldn't run ffprobe".into(), e))?;
    check_exit(&audio_probe_output)?;
    let audio_len: f64 = {
        let duration = String::from_utf8_lossy(&audio_probe_output.stdout)
            .trim()
            .to_string();
        duration.parse().map_err(|_| {
            LastLegendError::FFMPEG(format!("audio duration wasn't a float but: {}", duration))
        })?
    };

    // Run FFMPEG command to taper the end since most rolls are intended to "loop forever".
    let ffmpeg_args = ArgBuilder::new()
        .add_all(GENERAL_FFMPEG_INSTRUCTIONS)
        .add_all(get_ffmpeg_loglevel())
        .add("-y")
        .add_kv("-i", looped_cache_file.path())
        .add_kv(
            "-af",
            format!("afade=t=out:st={}:d=5", (audio_len - 5f64).max(0f64)),
        )
        .add_kv("-f", "flac")
        .add(original_cache_file.path())
        .into_vec();
    log::debug!("Running ffmpeg {:?}", ffmpeg_args);
    let ffmpeg_taper_output = Command::new("ffmpeg")
        .args(ffmpeg_args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .output()
        .map_err(|e| LastLegendError::Io("Couldn't run ffmpeg".into(), e))?;
    check_exit(&ffmpeg_taper_output)?;

    std::io::copy(
        &mut File::open(original_cache_file.path())
            .map_err(|e| LastLegendError::Io("Couldn't open original cache file".into(), e))?,
        &mut output,
    )
    .map_err(|e| LastLegendError::Io("Couldn't copy from original cache file".into(), e))?;

    Ok(())
}

pub fn ogg_to_flac(
    mut reader: impl Read + Send,
    mut output: impl Write + Send,
) -> Result<(), LastLegendError> {
    let mut output_temp = tempfile::NamedTempFile::new()
        .map_err(|e| LastLegendError::Io("Couldn't create temporary cache file".into(), e))?;
    let ffmpeg_args = ArgBuilder::new()
        .add_all(GENERAL_FFMPEG_INSTRUCTIONS)
        .add_all(get_ffmpeg_loglevel())
        .add("-y")
        .add_kv("-i", "pipe:")
        .add_kv("-map_metadata", "0:s:a:0")
        .add_kv("-f", "flac")
        .add(output_temp.path())
        .into_vec();
    log::debug!("Running ffmpeg {:?}", ffmpeg_args);
    let mut child = ChildDropGuard(
        Command::new("ffmpeg")
            .args(ffmpeg_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| LastLegendError::Io("Couldn't spawn ffmpeg".into(), e))?,
    );
    let (stdout, stderr) = std::thread::scope(|s| {
        let mut stdin = child.stdin.take().unwrap();
        let to_ffmpeg = s.spawn(move || {
            std::io::copy(&mut reader, &mut stdin)
                .map_err(|e| LastLegendError::Io("Couldn't copy to ffmpeg".into(), e))?;
            Ok::<(), LastLegendError>(())
        });
        let mut stdout = child.stdout.take().unwrap();
        let stdout_task = s.spawn(move || {
            let mut stdout_buffer = Vec::new();
            std::io::copy(&mut stdout, &mut stdout_buffer)
                .map_err(|e| LastLegendError::Io("Couldn't copy stdout from ffmpeg".into(), e))?;
            Ok::<_, LastLegendError>(stdout_buffer)
        });
        let mut stderr = child.stderr.take().unwrap();
        let stderr_task = s.spawn(move || {
            let mut stderr_buffer = Vec::new();
            std::io::copy(&mut stderr, &mut stderr_buffer)
                .map_err(|e| LastLegendError::Io("Couldn't copy stderr from ffmpeg".into(), e))?;
            Ok::<_, LastLegendError>(stderr_buffer)
        });
        to_ffmpeg.join().expect("join error")?;
        let stdout = stdout_task.join().expect("join error")?;
        let stderr = stderr_task.join().expect("join error")?;

        Ok::<_, LastLegendError>((stdout, stderr))
    })?;
    let exit = child
        .0
        .wait()
        .map_err(|e| LastLegendError::Io("Couldn't wait for ffmpeg".into(), e))?;
    check_exit(&Output {
        status: exit,
        stderr,
        stdout,
    })?;

    std::io::copy(output_temp.as_file_mut(), &mut output)
        .map_err(|e| LastLegendError::Io("Couldn't copy from temp file".into(), e))?;
    Ok(())
}

fn get_ffmpeg_loglevel() -> [&'static str; 2] {
    match log::max_level() {
        log::LevelFilter::Trace => ["-loglevel", "debug"],
        _ => ["-loglevel", "error"],
    }
}

fn check_exit(output: &Output) -> Result<(), LastLegendError> {
    if !output.status.success() {
        return Err(LastLegendError::FFMPEG(format!(
            "exit code {}, {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(())
}

struct ChildDropGuard(Child);
impl Drop for ChildDropGuard {
    fn drop(&mut self) {
        match self.0.kill() {
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::InvalidInput => {}
            Err(e) => panic!("kill failed: {}", e),
        }
    }
}

impl Deref for ChildDropGuard {
    type Target = Child;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ChildDropGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
