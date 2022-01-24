use std::io::{ErrorKind, Read, Write};
use std::ops::{Deref, DerefMut};
use std::process::{Child, Command, Output, Stdio};

use crate::error::LastLegendError;
use crate::tricks::parse_command_args;

/// Loop an OGG into a FLAC using the Loopstart and Loopend metadata.
pub fn loop_using_metadata(
    mut reader: impl Read + Send + Unpin + 'static,
    mut output: impl Write + Send + Unpin + 'static,
) -> Result<(), LastLegendError> {
    const GENERAL_FFMPEG_INSTRUCTIONS: &str = "-hide_banner";

    let original_cache_file = tempfile::NamedTempFile::new()
        .map_err(|e| LastLegendError::Io("Couldn't create temporary cache file".into(), e))?;
    let looped_cache_file = tempfile::NamedTempFile::new()
        .map_err(|e| LastLegendError::Io("Couldn't create temporary loop cache file".into(), e))?;
    // Run FFMPEG command to fix the OGG.
    let ffmpeg_args = parse_command_args(&*format!(
        "{general} {loglevel} -y -i pipe: -map_metadata 0:s:a:0 -f flac {file}",
        general = GENERAL_FFMPEG_INSTRUCTIONS,
        loglevel = get_ffmpeg_loglevel(),
        file = original_cache_file.path().display(),
    ));
    log::debug!("Running ffmpeg {:?}", ffmpeg_args);
    let mut child = ChildDropGuard(
        Command::new("ffmpeg")
            .args(ffmpeg_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .map_err(|e| LastLegendError::Io("Couldn't spawn ffmpeg".into(), e))?,
    );
    let push_r = std::io::copy(&mut reader, &mut child.stdin.take().unwrap());
    let exit = child
        .0
        .wait()
        .map_err(|e| LastLegendError::Io("Couldn't wait for ffmpeg".into(), e))?;
    check_exit(&Output {
        status: exit,
        stderr: Vec::new(),
        stdout: Vec::new(),
    })?;
    // Throw copy errors after child error,
    // since if the child crashes its error is more meaningful.
    push_r.map_err(|e| LastLegendError::Io("Couldn't pull data from ffmpeg".into(), e))?;

    // Run FFMPEG command to tell me what the loop points are
    let probe_args = parse_command_args(&*format!(
        "{general} {loglevel} -i {file} -show_entries format_tags -of compact=p=0:nk=1",
        general = GENERAL_FFMPEG_INSTRUCTIONS,
        loglevel = get_ffmpeg_loglevel(),
        file = original_cache_file.path().display(),
    ));
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
        let [loop_start, loop_end] = match output.as_slice() {
            &[a, b, ..] => [a, b],
            _ => return Err(LastLegendError::FFMPEG(format!("bad output: {}", stdout))),
        };
        let loop_start = loop_start.parse().map_err(|_| {
            LastLegendError::FFMPEG(format!("audio loop_start wasn't a u32 but: {}", loop_start))
        })?;
        let loop_end = loop_end.parse().map_err(|_| {
            LastLegendError::FFMPEG(format!("audio duration wasn't a u32 but: {}", loop_end))
        })?;
        (loop_start, loop_end)
    };

    // Run FFMPEG command to loop the audio
    let ffmpeg_args = parse_command_args(&*format!(
        "{general} {loglevel} -y -i {file} -af aloop=loop=1:start={loop_start}:size={loop_size} -f flac {file2}",
        general = GENERAL_FFMPEG_INSTRUCTIONS,
        loglevel = get_ffmpeg_loglevel(),
        file = original_cache_file.path().display(),
        loop_start = loop_start,
        loop_size = loop_end - loop_start,
        file2 = looped_cache_file.path().display(),
    ));
    log::debug!("Running ffmpeg {:?}", ffmpeg_args);
    let ffmpeg_loop_output = Command::new("ffmpeg")
        .args(ffmpeg_args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .output()
        .map_err(|e| LastLegendError::Io("Couldn't run ffmpeg".into(), e))?;
    check_exit(&ffmpeg_loop_output)?;

    // Run FFMPEG command to tell me what the length is
    let probe_args = parse_command_args(&*format!(
        "{general} {loglevel} -i {file} -show_entries stream=duration -of compact=p=0:nk=1",
        general = GENERAL_FFMPEG_INSTRUCTIONS,
        loglevel = get_ffmpeg_loglevel(),
        file = looped_cache_file.path().display(),
    ));
    log::debug!("Running ffprobe {:?}", probe_args);
    let audio_probe_output = Command::new("ffprobe")
        .args(probe_args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .output()
        .map_err(|e| LastLegendError::Io("Couldn't run ffprobe".into(), e))?;
    check_exit(&audio_probe_output)?;
    let audio_len: f64 = {
        let duration = String::from_utf8_lossy(&audio_probe_output.stdout).trim().to_string();
        duration.parse().map_err(|_| {
            LastLegendError::FFMPEG(format!("audio duration wasn't a float but: {}", duration))
        })?
    };

    // Run FFMPEG command to taper the end since most rolls are intended to "loop forever".
    let ffmpeg_args = parse_command_args(&*format!(
        "{general} {loglevel} -i {file} -af afade=t=out:st={start}:d=5 -f flac pipe:",
        general = GENERAL_FFMPEG_INSTRUCTIONS,
        loglevel = get_ffmpeg_loglevel(),
        file = looped_cache_file.path().display(),
        start = audio_len - 5f64,
    ));
    log::debug!("Running ffmpeg {:?}", ffmpeg_args);
    let mut child = ChildDropGuard(
        Command::new("ffmpeg")
            .args(ffmpeg_args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| LastLegendError::Io("Couldn't spawn ffmpeg".into(), e))?,
    );

    let mut child_stdout = child.0.stdout.take().unwrap();
    let pull_r = std::io::copy(&mut child_stdout, &mut output);
    let exit = child
        .0
        .wait()
        .map_err(|e| LastLegendError::Io("Couldn't wait for ffmpeg".into(), e))?;
    check_exit(&Output {
        status: exit,
        stderr: Vec::new(),
        stdout: Vec::new(),
    })?;
    // Throw copy errors after child error,
    // since if the child crashes its error is more meaningful.
    pull_r.map_err(|e| LastLegendError::Io("Couldn't pull data from ffmpeg".into(), e))?;

    Ok(())
}

fn get_ffmpeg_loglevel() -> &'static str {
    match log::max_level() {
        log::LevelFilter::Trace => "-loglevel debug",
        _ => "-loglevel error",
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
