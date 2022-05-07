use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BGM {
    pub file: String,
    pub priority: u8,
    pub disable_restart_timeout: bool,
    pub disable_restart: bool,
    pub pass_end: bool,
    pub disable_restart_reset_time: f32,
    pub special_mode: u8,
}
