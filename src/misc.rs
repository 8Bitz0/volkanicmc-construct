use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum FsObjectType {
    None,
    File,
    Directory,
}

impl std::fmt::Display for FsObjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub async fn fs_obj<T: AsRef<Path>>(path: T) -> FsObjectType {
    if path.as_ref().is_file() {
        FsObjectType::File
    } else if path.as_ref().is_dir() {
        FsObjectType::Directory
    } else {
        FsObjectType::None
    }
}
