use crate::resources::Jdk;
use crate::vkstore;

use super::{BuildError, misc::DownloadError, misc::download};

pub async fn prepare_jdk(store: vkstore::VolkanicStore, jdk: Jdk) -> Result<(), BuildError> {
    todo!()
}
