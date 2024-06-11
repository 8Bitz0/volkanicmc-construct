use indicatif::ProgressBar;
use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
};
use tracing::{debug, error};

use crate::resources::style;

use super::Error;

pub async fn export<P: AsRef<Path>>(
    build_path: P,
    objects: &HashMap<String, PathBuf>,
    path: P,
) -> Result<(), Error> {
    // Check if the build directory exists
    if !build_path.as_ref().is_dir() {
        return Err(Error::DirectoryMissing(path.as_ref().to_path_buf()));
    }

    // Check that all files in export info exist
    for f in objects {
        let file_path = build_path.as_ref().join(f.1);

        debug!("Checking export object \"{}\"", file_path.to_string_lossy());
        if !file_path.exists() {
            error!(
                "Could not find object to export: \"{}\"",
                file_path.to_string_lossy()
            );
            return Err(Error::ExportableNotFound(file_path));
        }
    }

    let mut f = File::create(&path).map_err(Error::Filesystem)?;
    let mut builder = tar::Builder::new(&mut f);

    let progress = ProgressBar::new(objects.len() as u64)
        .with_style(style::get_pb_style(style::ProgressStyleType::Export).await);

    // Append all files to the archive
    for f in objects {
        let msg = format!("{}", f.1.to_string_lossy());
        progress.set_message(msg);

        let file_path = build_path.as_ref().join(f.1);
        let mut file = File::open(&file_path).map_err(Error::Filesystem)?;

        debug!("Appending object \"{}\"", file_path.to_string_lossy());
        builder
            .append_file(f.0, &mut file)
            .map_err(Error::TarBuilder)?;

        progress.inc(1);
    }

    progress.set_message("Writing export info...");
    // Make JSON out of objects
    let export_json = serde_jsonc::to_string_pretty(objects).map_err(Error::Json)?;
    let export_json_bytes = export_json.as_bytes();

    // Create a header for the archive entry
    let mut header = tar::Header::new_gnu();
    // The file modification date must be set manually
    let now = std::time::SystemTime::now();
    header.set_mtime(match now.duration_since(std::time::UNIX_EPOCH) {
        Ok(o) => o.as_secs(),
        Err(_) => {
            error!("Failed to get time duration since Unix epoch (which is probably bad). The export info will appear to be from 1970.");

            0
        }
    });
    header.set_size(export_json_bytes.len() as u64);
    header.set_cksum();

    // Write the data
    builder
        .append_data(
            &mut header,
            PathBuf::from(super::EXPORT_INFO_FILENAME),
            export_json_bytes,
        )
        .map_err(Error::TarBuilder)?;

    progress.set_message("Finishing...");
    builder.finish().map_err(Error::TarBuilder)?;

    progress.finish_with_message("Export finished");

    Ok(())
}
