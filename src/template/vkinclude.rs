use std::path;

const INCLUDE_DIR_NAME: &str = ".vkinclude";

#[derive(Clone, Debug)]
pub struct VolkanicInclude {
    path: path::PathBuf,

}

impl VolkanicInclude {
    pub async fn new() -> Self {
        Self {
            path: path::PathBuf::from(INCLUDE_DIR_NAME),
        }
    }
    pub fn get(&self, id: impl std::fmt::Display) -> Option<path::PathBuf> {
        let p = self.path.join(id.to_string());

        if p.exists() {
            Some(p)
        } else {
            None
        }
    }
}
