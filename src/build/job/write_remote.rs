use futures_util::TryFutureExt;
use std::path::Path;
use tokio::fs;
use tracing::{debug, error, info};

use crate::build::misc;
use crate::fsobj;
use crate::template::resource::ArchiveInfo;
use crate::vkstore::VolkanicStore;

use super::Error;

pub async fn write_remote<P: AsRef<Path>, T: std::fmt::Display>(
    store: &VolkanicStore,
    template_path: P,
    archive: Option<&ArchiveInfo>,
    url: T,
    sha512: Option<T>,
    user_agent: Option<T>,
    override_name: Option<T>,
) -> Result<(), Error> {
    let abs_path = store.build_path.join(template_path);

    fsobj::create_ancestors(&abs_path)
        .await
        .map_err(Error::CreateFilesystemAncestors)?;

    let name = {
        match override_name {
            Some(name) => name.to_string(),
            None => {
                if let Some(name) = misc::get_remote_filename(url.to_string()).await {
                    name
                } else {
                    return Err(Error::NoFileNameInPath(abs_path));
                }
            }
        }
    };

    let p = misc::download_progress(
        store.clone(),
        url,
        match sha512 {
            Some(sha512) => misc::Verification::Sha512(sha512.to_string()),
            None => misc::Verification::None,
        },
        name,
        user_agent,
    )
    .map_err(Error::Download)
    .await?;

    match archive {
        Some(t) => {
            let archive_path = misc::extract(store.clone(), p, t.archive_format.clone())
                .await
                .map_err(Error::Extraction)?;
            let a_path_inner = archive_path.join(t.inner_path.clone());

            match fsobj::fs_obj(a_path_inner.clone()).await {
                fsobj::FsObjectType::Directory => {
                    match copy_dir::copy_dir(&a_path_inner, &abs_path) {
                        Ok(_) => {
                            info!(
                                "Copied resource directory \"{}\" to \"{}\"",
                                a_path_inner.to_string_lossy(),
                                abs_path.to_string_lossy()
                            );
                        }
                        Err(e) => {
                            debug!("Errors ocurred during JDK copy: {:#?}", e);
                            return Err(Error::DirectoryCopyFailed(a_path_inner));
                        }
                    }

                    for p in &t.post_remove {
                        let abs_rm_path = abs_path.join(p);

                        match fsobj::fs_obj(abs_rm_path.clone()).await {
                            fsobj::FsObjectType::Directory => {
                                info!("Remove post-removal directory: \"{}\"", p.to_string_lossy());
                                fs::remove_dir_all(abs_rm_path)
                                    .await
                                    .map_err(Error::Filesystem)?;
                            }
                            fsobj::FsObjectType::File => fs::remove_file(abs_rm_path)
                                .await
                                .map_err(Error::Filesystem)?,
                            fsobj::FsObjectType::None => {
                                error!(
                                    "Post-removal inner-archive path not found: {}",
                                    p.to_string_lossy()
                                );
                            }
                        }
                    }
                }
                fsobj::FsObjectType::File => {
                    fs::copy(&a_path_inner, &abs_path)
                        .await
                        .map_err(Error::Filesystem)?;
                }
                fsobj::FsObjectType::None => {
                    return Err(Error::InnerArchivePathNotFound(a_path_inner))
                }
            }
        }
        None => {
            fs::copy(p, abs_path).await.map_err(Error::Filesystem)?;
        }
    }

    Ok(())
}
