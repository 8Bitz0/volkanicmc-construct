use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tokio::{fs, io::AsyncWriteExt};
use tracing::debug;

use super::PersistentObject;

const MAX_RECURSION_DEPTH: usize = 128;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PersistInfo {
    name: String,
    objects: HashMap<PathBuf, PersistentObject>,
}

#[derive(Debug)]
pub struct PersistInfoFile {
    info: PersistInfo,
    path: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum PersistentInfoError {
    #[error("Filesystem error: {0}")]
    Filesystem(std::io::Error),
    #[error("Failed to serialize JSON: {0}")]
    JsonSerialize(serde_jsonc::Error),
}

impl PersistInfoFile {
    pub async fn new<Q: AsRef<Path>, P: std::fmt::Display>(path: Q, name: P) -> Self {
        PersistInfoFile {
            info: PersistInfo {
                name: name.to_string(),
                objects: HashMap::new(),
            },
            path: path.as_ref().to_path_buf(),
        }
    }
    pub async fn add<T: AsRef<Path>>(
        &mut self,
        path: T,
        obj: PersistentObject,
    ) -> Result<(), PersistentInfoError> {
        // self.info.objects.insert(path.as_ref().to_path_buf(), obj);

        match obj {
            PersistentObject::Directory(inner) => {
                self.info
                    .objects
                    .extend({ copy::export(path.as_ref(), &self.path, inner, 0).await? });
            }
            PersistentObject::File(inner) => {
                self.info
                    .objects
                    .extend({ copy::export(path.as_ref(), &self.path, inner, 0).await? });
            }
        }

        Ok(())
    }
    pub async fn update(&self) -> Result<(), PersistentInfoError> {
        debug!(
            "Writing to persist file at: \"{}\"",
            &self.path.join("persist.json").to_string_lossy()
        );

        let mut f = fs::File::create(&self.path.join("persist.json"))
            .await
            .map_err(PersistentInfoError::Filesystem)?;

        let persist_json =
            serde_jsonc::to_string(&self.info).map_err(PersistentInfoError::JsonSerialize)?;

        let json_bytes = persist_json.as_bytes();

        f.write_all(json_bytes)
            .await
            .map_err(PersistentInfoError::Filesystem)?;

        debug!(
            "Wrote {} bytes to \"{}\"",
            json_bytes.len(),
            &self.path.to_string_lossy()
        );

        Ok(())
    }
}

mod copy {
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
    };
    use tokio::fs;
    use tracing::{debug, error};

    use crate::{
        misc::{fs_obj, FsObjectType},
        persistence::{info::MAX_RECURSION_DEPTH, PersistentObject},
    };

    use super::PersistentInfoError;

    pub async fn export<Q: AsRef<Path>, P: AsRef<Path>, S: AsRef<Path>>(
        from: Q,
        pers_path: P,
        inner_path: S,
        index: usize,
    ) -> Result<HashMap<PathBuf, PersistentObject>, PersistentInfoError> {
        if index >= MAX_RECURSION_DEPTH {
            panic!("Max recursion depth reached");
        }

        let f_uuid = uuid::Uuid::new_v4();

        let mut objects: HashMap<PathBuf, PersistentObject> = HashMap::new();

        let path = pers_path.as_ref().join(f_uuid.to_string());

        match fs_obj(&from).await {
            FsObjectType::Directory => {
                for p in from
                    .as_ref()
                    .read_dir()
                    .map_err(PersistentInfoError::Filesystem)
                    .into_iter()
                    .flatten()
                {
                    let subobj = p.map_err(PersistentInfoError::Filesystem)?;
                    let path = subobj.path();

                    objects.extend(
                        export(path, pers_path.as_ref(), inner_path.as_ref(), index + 1).await?,
                    );
                }
            }
            FsObjectType::File => {
                fs::copy(&from, &path)
                    .await
                    .map_err(PersistentInfoError::Filesystem)?;

                objects.insert(
                    PathBuf::from(f_uuid.to_string()),
                    PersistentObject::File(
                        path.strip_prefix(pers_path.as_ref()).unwrap().to_path_buf(),
                    ),
                );
            }
            FsObjectType::None => {
                error!(
                    "Persistent file not found: \"{}\"",
                    inner_path.as_ref().to_string_lossy()
                );
            }
        }

        debug!(
            "Exporting file \"{}\" with UUID \"{}\" to \"{}\"",
            inner_path.as_ref().to_string_lossy(),
            f_uuid.to_string(),
            path.to_string_lossy(),
        );

        Ok(objects)
    }
}
