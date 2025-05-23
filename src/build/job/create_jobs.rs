use tracing::error;

use crate::resources::JdkLookup;
use crate::template::{self, vkinclude};

use super::{Job, JobAction, Error};

pub async fn create_jobs(
    template: &crate::template::Template,
    overlays: &Vec<template::overlay::Overlay>,
    jdk_lookup: JdkLookup,
    var_map: &template::var::VarMap,
    no_verify: bool,
    force_jdk_distribution: Option<String>,
    preferred_distributions: Option<Vec<String>>,
) -> Result<Vec<Job>, Error> {
    let mut jobs = vec![];

    let mut overlay_runtime = None;

    for o in overlays {
        if let Some(r) = &o.runtime {
            if overlay_runtime.is_some() {
                error!("Only one overlay can set the runtime configuration at once");
                return Err(Error::ConflictingRuntimes);
            }

            overlay_runtime = Some(r.clone());
            break;
        }
    }
    
    let runtime = match overlay_runtime {
        Some(r) => r,
        None => template.runtime.clone(),
    };

    // Setup JDK
    match runtime {
        template::resource::ServerRuntimeResource::Jdk {
            version,
            jar_path: _,
            jdk_args: _,
            ..
        } => {
            jobs.push(Job {
                title: "Prepare JDK".into(),
                action: JobAction::PrepareJdk {
                    jdk: {
                        let jdk = match jdk_lookup.find(
                            &version,
                            None,
                            force_jdk_distribution,
                            preferred_distributions,
                        ).await {
                            Ok(jdk) => jdk,
                            Err(e) => {
                                error!("Failed to find JDK via Foojay Disco: {e}");
                                return Err(Error::DiscoLookup(e))
                            }
                        };

                        match jdk {
                            Some(jdk) => jdk,
                            None => {
                                error!("No JDK found for your system (version: {})", version);
                                return Err(Error::JdkNotFound(version.to_string()));
                            }
                        }
                    },
                    no_verify,
                },
            });
        }
    }

    let mut resources = template.resources.clone();

    for o in overlays {
        resources.extend(o.resources.clone());
    }

    // Setup additional resources
    for resource in resources {
        match resource {
            template::resource::GenericResource::Remote {
                url,
                user_agent,
                override_name,
                sha512,
                use_variables,
                archive,
                template_path: path,
            } => {
                jobs.push(Job {
                    title: "Download additional resource".into(),
                    action: JobAction::WriteFileRemote {
                        path: path.clone(),
                        url: url.clone(),
                        user_agent: user_agent.clone(),
                        override_name: override_name.clone(),
                        archive: archive.clone(),
                        sha512: sha512.clone(),
                    },
                });

                if let Some(use_variables) = use_variables {
                    if archive.is_some() {
                        error!("Variable substitution is not supported for archives");

                        return Err(Error::ArchivesCannotHaveVariables(
                            override_name
                                .clone()
                                .unwrap_or(path.clone().to_string_lossy().to_string())
                                .into(),
                        ));
                    }

                    jobs.push(Job {
                        title: "Perform variable substitution".into(),
                        action: JobAction::ProcessVariables {
                            path: path.clone(),
                            format: use_variables.clone(),
                            variables: var_map.clone(),
                        },
                    })
                }
            }
            template::resource::GenericResource::Base64 {
                base64: base,
                use_variables,
                template_path,
            } => {
                jobs.push(Job {
                    title: "Write file from Base64".into(),
                    action: JobAction::WriteFileBase64 {
                        path: template_path.clone(),
                        contents: base.clone(),
                    },
                });

                if let Some(use_variables) = use_variables {
                    jobs.push(Job {
                        title: "Perform variable substitution".into(),
                        action: JobAction::ProcessVariables {
                            path: template_path.clone(),
                            format: use_variables.clone(),
                            variables: var_map.clone(),
                        },
                    })
                }
            }
            template::resource::GenericResource::Include {
                include_id,
                use_variables,
                template_path,
            } => {
                // Pre-checks
                let include = vkinclude::VolkanicInclude::new().await;
                if include.get(&include_id).await.is_none() {
                    error!("Did not find \"{}\" in include directory.", include_id);
                    return Err(Error::NotAvailableInIncludeFolder(
                        include_id.to_string(),
                    ));
                }

                // Push job
                jobs.push(Job {
                    title: "Copy additional resource".into(),
                    action: JobAction::CopyFromInclude {
                        id: include_id.to_string(),
                        template_path: template_path.clone(),
                    },
                });

                if let Some(use_variables) = use_variables {
                    jobs.push(Job {
                        title: "Perform variable substitution".into(),
                        action: JobAction::ProcessVariables {
                            path: template_path.clone(),
                            format: use_variables.clone(),
                            variables: var_map.clone(),
                        },
                    })
                }
            }
        }
    }

    Ok(jobs)
}
