use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::{RwLock, RwLockUpgradableReadGuard};

use crate::data::index2::Index2;
use crate::error::LastLegendError;
use crate::sqpath::SqPath;

/// Entry point for loading FFXIV data.
/// This is best to use at a high level, as it caches the data from disk.
#[derive(Debug, Clone)]
pub struct Repository {
    repo_path: PathBuf,
    state: Arc<RwLock<RepoState>>,
}

impl Repository {
    pub fn new(repo_path: PathBuf) -> Self {
        Self {
            repo_path,
            state: Arc::new(RwLock::new(RepoState {
                indexes: HashMap::new(),
            })),
        }
    }

    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }

    pub fn get_index_for<F: AsRef<SqPath>>(
        &self,
        file_name: F,
    ) -> Result<Arc<Index2>, LastLegendError> {
        let file_name = file_name.as_ref().to_owned();
        let index_path = file_name
            .sqpack_index_path(&self.repo_path)
            .ok_or_else(|| LastLegendError::InvalidSqPath(file_name.as_str().to_string()))?;

        self.load_index_file(index_path.into())
    }

    pub fn load_index_file(&self, index_path: Cow<Path>) -> Result<Arc<Index2>, LastLegendError> {
        // Pass one: check with read lock.
        {
            let state = self.state.read();
            if let Some(v) = state.indexes.get(index_path.as_ref()) {
                return Ok(Arc::clone(v));
            }
        }

        // Pass two: try again with upgradable read lock.
        let state = self.state.upgradable_read();
        if let Some(v) = state.indexes.get(index_path.as_ref()) {
            return Ok(Arc::clone(v));
        }
        // Pass three: load it under upgradable read lock, and then write lock to save it.
        let index2 = Arc::new(Index2::load_from_path(&index_path)?);
        let mut state = RwLockUpgradableReadGuard::upgrade(state);
        state
            .indexes
            .insert(index_path.into_owned(), Arc::clone(&index2));
        Ok(index2)
    }
}

#[derive(Debug)]
struct RepoState {
    indexes: HashMap<PathBuf, Arc<Index2>>,
}
