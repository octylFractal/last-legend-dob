use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct OrchestrionPath {
    pub file_name: String,
}
