use indicatif::ProgressBar;
use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use crate::{fsobj, resources::style};

use super::{Error, EXPORT_INFO_FILENAME};

pub async fn import<P: AsRef<Path>>(build_path: P, archive_path: P) -> Result<(), Error> {
    let progress = ProgressBar::new(1)
        .with_style(style::get_pb_style(style::ProgressStyleType::Import).await)
        .with_message("Checking archive...");

    let info = {
        // Start checking the archive
        let f = File::open(archive_path.as_ref()).map_err(Error::Filesystem)?;
        let mut archive = tar::Archive::new(f);

        let mut info_f = match find_file(&mut archive, EXPORT_INFO_FILENAME).await {
            Ok(f) => f,
            Err(e) => match e {
                Error::NotInArchive(_) => {
                    return Err(Error::NotAnExportArchive);
                }
                _ => {
                    return Err(e);
                }
            },
        };

        let mut info_plain = String::new();
        info_f
            .read_to_string(&mut info_plain)
            .map_err(Error::Filesystem)?;

        serde_jsonc::from_str::<HashMap<String, PathBuf>>(&info_plain).map_err(Error::Json)?
    };

    progress.set_length(info.len() as u64);

    for obj in info {
        progress.set_message(format!("{}", obj.1.to_string_lossy()));

        let f = File::open(archive_path.as_ref()).map_err(Error::Filesystem)?;
        let mut archive = tar::Archive::new(f);

        // The reader must be reopened each time to move the position back to 0
        let mut archive_f = match find_file(&mut archive, obj.0).await {
            Ok(f) => f,
            Err(e) => match e {
                Error::NotInArchive(_) => {
                    return Err(Error::ExportableNotFound(obj.1));
                }
                _ => {
                    return Err(e);
                }
            },
        };

        let to_path = build_path.as_ref().to_path_buf().join(obj.1);
        fsobj::create_ancestors(&to_path)
            .await
            .map_err(Error::CreateAncestor)?;

        let mut to_f = File::create(to_path).map_err(Error::Filesystem)?;

        std::io::copy(&mut archive_f, &mut to_f).map_err(Error::Filesystem)?;

        progress.inc(1);
    }

    progress.finish_with_message("Import complete");

    Ok(())
}

async fn find_file<P: AsRef<Path>>(
    archive: &mut tar::Archive<File>,
    file_path: P,
) -> Result<tar::Entry<File>, Error> {
    let entries = archive.entries_with_seek().map_err(Error::TarReader)?;

    // Iterate through each archive entry
    for entry in entries {
        let entry = entry.map_err(Error::TarReader)?;

        if entry.path().map_err(Error::TarReader)? == file_path.as_ref() {
            return Ok(entry);
        }
    }

    Err(Error::NotInArchive(file_path.as_ref().to_path_buf()))
}
