use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Orchestrion {
    pub name: String,
    pub description: String,
}
