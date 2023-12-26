use base64::Engine;

use tokio::{fs, io::AsyncReadExt};
use tracing::error;

use crate::resources;

use super::{resource, vkinclude, Template};

#[derive(Debug, thiserror::Error)]
pub enum TemplateManagementError {
    #[error("Filesystem error: {0}")]
    Filesystem(std::io::Error),
}

pub async fn embed(template: Template) -> Result<Template, TemplateManagementError> {
    let mut new_template = template;

    for r in &mut new_template.resources {
        match r {
            resource::GenericResource::Include {
                include_id,
                variable_substitution,
                template_path,
            } => {
                let include = vkinclude::VolkanicInclude::new().await;
                let p = match include.get(&include_id) {
                    Some(p) => p,
                    None => {
                        error!(
                            "No include found for ID: \"{}\", skipping resource...",
                            &include_id
                        );
                        continue;
                    }
                };

                let mut f = fs::File::open(p)
                    .await
                    .map_err(TemplateManagementError::Filesystem)?;
                let mut buffer = [0; resources::conf::FILE_BUFFER_SIZE];

                let mut f_contents: Vec<u8> = vec![];

                loop {
                    let bytes_read = f
                        .read(&mut buffer)
                        .await
                        .map_err(TemplateManagementError::Filesystem)?;

                    if bytes_read == 0 {
                        break;
                    }

                    f_contents.append(&mut buffer[..bytes_read].to_vec());
                }

                let base64_config = base64::engine::GeneralPurposeConfig::new();
                let base64_engine =
                    base64::engine::GeneralPurpose::new(&base64::alphabet::STANDARD, base64_config);

                *r = resource::GenericResource::Base64 {
                    base64: base64_engine.encode(&f_contents),
                    variable_substitution: *variable_substitution,
                    template_path: template_path.clone(),
                };
            }
            _ => continue,
        }
    }

    Ok(new_template)
}
