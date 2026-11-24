use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use hg_contracts::ids::{EpisodeKey, SeriesId};

use crate::error::InfraError;

const REMOVE_RETRY_DELAYS_MS: [u64; 5] = [20, 40, 80, 160, 320];

#[derive(Debug, Clone)]
pub struct StorageLayout {
    catalog_cache_dir: PathBuf,
    playback_cache_dir: PathBuf,
    offline_staging_dir: PathBuf,
    offline_library_dir: PathBuf,
}

impl StorageLayout {
    pub fn new(data_root: &Path, cache_root: &Path) -> Result<Self, InfraError> {
        let offline_root = data_root.join("offline");
        if paths_overlap(&offline_root, cache_root) {
            return Err(InfraError::InvalidPath {
                detail: "离线片库目录严禁与缓存根目录重叠".to_owned(),
            });
        }

        Ok(Self {
            catalog_cache_dir: cache_root.join("catalog"),
            playback_cache_dir: cache_root.join("playback"),
            offline_staging_dir: offline_root.join("staging"),
            offline_library_dir: offline_root.join("library"),
        })
    }

    pub async fn ensure_dirs(&self) -> Result<(), InfraError> {
        tokio::fs::create_dir_all(&self.catalog_cache_dir).await?;
        tokio::fs::create_dir_all(&self.playback_cache_dir).await?;
        tokio::fs::create_dir_all(&self.offline_staging_dir).await?;
        tokio::fs::create_dir_all(&self.offline_library_dir).await?;
        Ok(())
    }

    #[must_use]
    pub fn catalog_cache_dir(&self) -> &Path {
        &self.catalog_cache_dir
    }

    #[must_use]
    pub fn playback_cache_dir(&self) -> &Path {
        &self.playback_cache_dir
    }

    #[must_use]
    pub fn offline_staging_dir(&self) -> &Path {
        &self.offline_staging_dir
    }

    #[must_use]
    pub fn offline_library_dir(&self) -> &Path {
        &self.offline_library_dir
    }

    #[must_use]
    pub fn staging_episode_dir(&self, key: EpisodeKey) -> PathBuf {
        self.offline_staging_dir
            .join(format!("s{}_e{}", key.series_id, key.ep_num))
    }

    #[must_use]
    pub fn library_series_dir(&self, series_id: SeriesId) -> PathBuf {
        self.offline_library_dir.join(format!("s{series_id}"))
    }

    #[must_use]
    pub fn library_episode_dir(&self, key: EpisodeKey) -> PathBuf {
        self.library_series_dir(key.series_id)
            .join(format!("e{}", key.ep_num))
    }

    pub fn resolve_offline_library_path(&self, rel_path: &str) -> Result<PathBuf, InfraError> {
        let relative = validate_relative_path(rel_path)?;
        Ok(self.offline_library_dir.join(relative))
    }

    pub async fn commit_staging_episode(&self, key: EpisodeKey) -> Result<PathBuf, InfraError> {
        let staging_dir = self.staging_episode_dir(key);
        let series_dir = self.library_series_dir(key.series_id);
        let target_dir = self.library_episode_dir(key);

        tokio::fs::create_dir_all(&series_dir).await?;
        remove_dir_if_exists(&target_dir).await?;
        tokio::fs::rename(&staging_dir, &target_dir).await?;
        Ok(target_dir)
    }

    pub async fn clear_catalog_cache(&self) -> Result<u64, InfraError> {
        clear_isolated_cache_dir(&self.catalog_cache_dir).await
    }

    pub async fn clear_playback_cache(&self) -> Result<u64, InfraError> {
        clear_isolated_cache_dir(&self.playback_cache_dir).await
    }

    pub async fn remove_offline_rel_dir_with_retry(&self, rel_dir: &str) -> Result<(), InfraError> {
        let target = self.resolve_offline_library_path(rel_dir)?;
        remove_dir_with_retry(&target).await
    }

    pub async fn dir_size_bytes(root: &Path) -> Result<u64, InfraError> {
        if !tokio::fs::try_exists(root).await? {
            return Ok(0);
        }

        let mut total = 0_u64;
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let mut entries = tokio::fs::read_dir(&dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let metadata = entry.metadata().await?;
                if metadata.is_dir() {
                    stack.push(entry.path());
                } else if metadata.is_file() {
                    total = total.saturating_add(metadata.len());
                }
            }
        }
        Ok(total)
    }
}

fn paths_overlap(left: &Path, right: &Path) -> bool {
    left == right || left.starts_with(right) || right.starts_with(left)
}

fn validate_relative_path(raw: &str) -> Result<PathBuf, InfraError> {
    if raw.is_empty() || raw.contains('\\') {
        return Err(InfraError::InvalidPath {
            detail: format!("非法的相对路径: {raw}"),
        });
    }

    let mut clean = PathBuf::new();
    for component in Path::new(raw).components() {
        match component {
            Component::Normal(part) => clean.push(part),
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {
                return Err(InfraError::InvalidPath {
                    detail: format!("路径包含非法遍历段: {raw}"),
                });
            }
        }
    }

    if clean.as_os_str().is_empty() {
        return Err(InfraError::InvalidPath {
            detail: "相对路径不能为空".to_owned(),
        });
    }

    Ok(clean)
}

async fn clear_isolated_cache_dir(dir: &Path) -> Result<u64, InfraError> {
    let freed = StorageLayout::dir_size_bytes(dir).await?;
    remove_dir_if_exists(dir).await?;
    tokio::fs::create_dir_all(dir).await?;
    Ok(freed)
}

async fn remove_dir_if_exists(path: &Path) -> Result<(), InfraError> {
    match tokio::fs::remove_dir_all(path).await {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(InfraError::Io(err)),
    }
}

async fn remove_dir_with_retry(path: &Path) -> Result<(), InfraError> {
    let mut last_error: Option<std::io::Error> = None;

    for delay_ms in REMOVE_RETRY_DELAYS_MS {
        match tokio::fs::remove_dir_all(path).await {
            Ok(()) => return Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(err) => {
                last_error = Some(err);
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
        }
    }

    match tokio::fs::remove_dir_all(path).await {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(InfraError::Io(last_error.unwrap_or(err))),
    }
}
