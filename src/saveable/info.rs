use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tracing::{debug, error};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::vkstore::VolkanicStore;

use super::{export::export, Error};

// const INFO_FILE_NAME: &str = ".vkexport.json";

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct ExportInfo {
    build_path: PathBuf,
    objects: HashMap<String, PathBuf>,
}

impl ExportInfo {
    pub async fn new(store: VolkanicStore) -> Self {
        ExportInfo {
            build_path: store.build_path,
            objects: HashMap::new(),
        }
    }
    pub async fn add<P: AsRef<Path>>(&mut self, inner_path: P) -> Result<(), Error> {
        let path = self.build_path.join(inner_path.as_ref());

        if path.is_file() {
            self.objects.insert(
                Uuid::new_v4().to_string(),
                inner_path.as_ref().to_path_buf(),
            );
        } else if path.is_dir() {
            let dir = WalkDir::new(path);

            for f in dir {
                match f {
                    Ok(f) => {
                        let path = f.path().to_path_buf();

                        // Directories should be skipped
                        if path.is_dir() {
                            debug!("\"{}\" is a directory, skipping.", path.to_string_lossy());
                            continue;
                        }

                        // Remove the parent directories outside of the build directory.
                        //
                        // `unwrap()` is used here as removing the prefix can only throw
                        // an error if there's an issue within the source code.
                        let rel_path = path.strip_prefix(&self.build_path).unwrap();

                        self.objects
                            .insert(Uuid::new_v4().to_string(), rel_path.to_path_buf());
                    }
                    Err(e) => match e.path() {
                        Some(p) => {
                            error!(
                                "Error at \"{}\", got \"{}\"",
                                p.to_string_lossy(),
                                e.to_string()
                            );
                            return Err(Error::WalkDirFile {
                                path: p.to_path_buf(),
                                err: e.to_string(),
                            });
                        }
                        None => {
                            error!("Abrupt error \"{}\"", e.to_string());
                            return Err(Error::WalkDirAbrupt(e.to_string()));
                        }
                    },
                }
            }
        }

        Ok(())
    }
    pub async fn export(&self, path: PathBuf) -> Result<(), Error> {
        export(&self.build_path, &self.objects, &path).await?;

        Ok(())
    }
}
