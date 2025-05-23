// #![deny(warnings)]

use clap::{Parser, Subcommand};
use std::path;
use tracing::{debug, error, info};

mod build;
mod exec;
mod fsobj;
mod hostinfo;
mod resources;
mod saveable;
mod template;
mod vkstore;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
    /// Override store directory
    #[arg(short = 's', long)]
    override_store_dir: Option<path::PathBuf>,
    /// Override build directory
    #[arg(short = 'b', long)]
    override_build_dir: Option<path::PathBuf>,
    /// Override downloads directory
    #[arg(short = 'd', long)]
    override_downloads_dir: Option<path::PathBuf>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Build in the current directory using a template from the given path
    Build {
        path: path::PathBuf,
        #[arg(short = 'o', long)]
        overlay: Vec<path::PathBuf>,
        #[arg(short = 'f', long)]
        force: bool,
        #[arg(short = 'v', long)]
        user_vars: Vec<String>,
        /// Add additional JVM arguments to place before the template's JVM arguments
        #[arg(short = 'j', long, value_parser, num_args = 1.., value_delimiter = ' ')]
        additional_jvm_args: Vec<String>,
        /// Include a importable archive in the build
        #[arg(short = 'i', long)]
        import_save: Option<path::PathBuf>,
        /// Disable verification for all files
        #[arg(long)]
        no_verify: bool,
        /// Only allows a specific JDK distribution
        #[arg(long)]
        force_jdk_distribution: Option<String>,
    },
    /// Parse a template at the given path
    Check { path: path::PathBuf },
    /// Runs the build in the current directory. Only use for testing with trusted templates. Do not use for execution in production.
    Run,
    /// Template management commands
    #[command(subcommand)]
    Template(TemplateCommand),
    /// Create a Bash script from the execution information of an existing build
    ExecScript {
        format: exec::script::ExecScriptType,
    },
    Export {
        /// Path to write export archive
        path: path::PathBuf,
    },
    /// Clear downloads and temporary files
    Clean,
}

#[derive(Debug, Clone, Subcommand)]
enum TemplateCommand {
    /// Moves all external "include" files into template
    Embed { path: path::PathBuf },
    /// Prints a basic template
    Create,
    /// Generate a JSON schema for templates
    GenerateSchema,
    /// Overlay management commands
    #[command(subcommand)]
    Overlay(OverlayCommand),
}

#[derive(Debug, Clone, Subcommand)]
enum OverlayCommand {
    /// Prints a basic template overlay
    Create,
    /// Generate a JSON schema for template overlays
    GenerateSchema,
}

async fn init_log() {
    #[cfg(feature = "debug_log")]
    {
        println!("Debug logging enabled");
        tracing_subscriber::fmt()
            .event_format(tracing_subscriber::fmt::format().compact())
            .with_line_number(true)
            .with_max_level(tracing::Level::DEBUG)
            .init();
    }

    #[cfg(not(feature = "debug_log"))]
    {
        tracing_subscriber::fmt()
            .event_format(tracing_subscriber::fmt::format().compact())
            .with_target(false)
            .with_max_level(tracing::Level::INFO)
            .init();
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let store_d = args.override_store_dir;
    let build_d = args.override_build_dir;
    let downloads_d = args.override_downloads_dir;

    match args.command {
        Command::Build {
            path,
            overlay: overlay_paths,
            force,
            user_vars,
            additional_jvm_args,
            import_save,
            no_verify,
            force_jdk_distribution,
        } => {
            init_log().await;

            let template = parse_template(path).await;
            let mut overlays = vec![];

            for p in overlay_paths {
                match template::overlay::Overlay::import(p).await {
                    Ok(overlay) => {
                        info!("Overlay \"{}\" parsed correctly", overlay.name);

                        overlays.push(overlay);
                    }
                    Err(e) => {
                        error!("Failed to parse overlay: {}", e);
                        
                        std::process::exit(1);
                    }
                }
            }

            let store = vkstore_init(store_d, build_d, downloads_d).await;

            match build::build(
                template,
                overlays,
                store.clone(),
                force,
                user_vars,
                additional_jvm_args,
                no_verify,
                force_jdk_distribution,
            )
            .await
            {
                Ok(()) => {}
                Err(e) => {
                    error!("Failed to build template: {}", e);
                    std::process::exit(1);
                }
            };

            if let Some(archive_path) = import_save {
                info!(
                    "Import from \"{}\" requested",
                    archive_path.to_string_lossy()
                );
                match saveable::import(store.build_path, archive_path).await {
                    Ok(()) => {}
                    Err(e) => {
                        error!("Failed to import archive: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                debug!("No import requested.");
            }
        }
        Command::Check { path } => {
            init_log().await;

            parse_template(path).await;
        }
        Command::Run => {
            init_log().await;

            let store = vkstore_init(store_d, build_d, downloads_d).await;

            match exec::run(&store).await {
                Ok(()) => {}
                Err(e) => {
                    error!("Failed to execute template: {}", e);
                    std::process::exit(1);
                }
            };
        }
        Command::Template(command) => match command {
            TemplateCommand::Embed { path } => {
                let template = match template::Template::import(path).await {
                    Ok(template) => template,
                    Err(e) => {
                        init_log().await;

                        error!("Failed to parse template: {}", e);
                        std::process::exit(1);
                    }
                };

                match template::manage::embed(template).await {
                    Ok(t) => {
                        println!(
                            "{}",
                            match serde_jsonc::to_string_pretty(&t) {
                                Ok(json) => json,
                                Err(e) => {
                                    init_log().await;

                                    error!("Failed to serialize template: {}", e);
                                    std::process::exit(1);
                                }
                            }
                        );
                    }
                    Err(e) => {
                        init_log().await;

                        error!("Failed to embed template: {}", e);
                        std::process::exit(1);
                    }
                };
            }
            TemplateCommand::Create => {
                println!(
                    "{}",
                    match serde_jsonc::to_string_pretty(&template::Template::default()) {
                        Ok(json) => json,
                        Err(e) => {
                            init_log().await;

                            error!("Failed to serialize template: {}", e);
                            std::process::exit(1);
                        }
                    }
                );
            }
            TemplateCommand::GenerateSchema => {
                let schema = schemars::schema_for!(template::Template);

                println!("{}", serde_jsonc::to_string(&schema).unwrap());
            }
            TemplateCommand::Overlay(command) => match command {
                OverlayCommand::Create => {
                    println!(
                        "{}",
                        match serde_jsonc::to_string_pretty(&template::overlay::Overlay::default()) {
                            Ok(json) => json,
                            Err(e) => {
                                init_log().await;

                                error!("Failed to serialize overlay: {}", e);
                                std::process::exit(1);
                            }
                        }
                    );
                }
                OverlayCommand::GenerateSchema => {
                    let schema = schemars::schema_for!(template::overlay::Overlay);

                    println!("{}", serde_jsonc::to_string(&schema).unwrap());
                }
            },
        },
        Command::ExecScript { format } => {
            let store = vkstore_init(store_d, build_d, downloads_d).await;

            if !build::BuildInfo::exists(&store).await {
                init_log().await;

                error!("No build info found");
                std::process::exit(1);
            }

            let build_info = match build::BuildInfo::get(&store).await {
                Ok(build_info) => build_info,
                Err(e) => {
                    init_log().await;

                    error!("Failed to initialize build info: {}", e);
                    std::process::exit(1);
                }
            };

            let exec_info = match build_info.exec {
                Some(exec_info) => exec_info,
                None => {
                    init_log().await;

                    error!("No execution info provided in build info file");
                    std::process::exit(1);
                }
            };

            println!(
                "{}",
                exec::script::to_script(exec_info, store.build_path, format).await
            );
        }
        Command::Clean => {
            init_log().await;

            let store = vkstore_init(store_d, build_d, downloads_d).await;

            match vkstore::VolkanicStore::clear_downloads(&store).await {
                Ok(()) => {}
                Err(e) => {
                    error!("Failed to clean store: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Command::Export { path } => {
            init_log().await;

            let store = vkstore_init(store_d, build_d, downloads_d).await;

            let build_info = match build::BuildInfo::get(&store).await {
                Ok(build_info) => build_info,
                Err(e) => {
                    error!("Failed to initialize build info: {}", e);
                    std::process::exit(1);
                }
            };

            let mut export = saveable::ExportInfo::new(store).await;

            let mut saveables = build_info.template.saveables.clone();

            for o in build_info.overlays {
                saveables.extend(o.saveables);
            }

            for s in saveables {
                match export.add(s).await {
                    Ok(()) => {}
                    Err(e) => {
                        error!("Failed to export build: {}", e);

                        std::process::exit(1);
                    }
                };
            }

            export.export(path).await.unwrap();
        }
    }
}

async fn vkstore_init<S: AsRef<path::Path>>(
    store_dir: Option<S>,
    build_dir: Option<path::PathBuf>,
    downloads_dir: Option<path::PathBuf>,
) -> vkstore::VolkanicStore {
    let mut store = match store_dir {
        Some(s) => vkstore::VolkanicStore::new_custom_root(s).await,
        None => vkstore::VolkanicStore::new().await,
    };

    if let Some(b) = build_dir {
        store = store.override_build(b).await;
    }

    if let Some(d) = downloads_dir {
        store = store.override_downloads(d).await;
    }

    match store.init().await {
        Ok(store) => store,
        Err(e) => {
            error!("Failed to initialize store: {}", e);
            std::process::exit(1);
        }
    };

    store
}

async fn parse_template<P: AsRef<path::Path>>(path: P) -> template::Template {
    match template::Template::import(path.as_ref()).await {
        Ok(template) => {
            info!("Template \"{}\" parsed correctly", template.name);
            template
        }
        Err(e) => {
            error!("Failed to parse template: {}", e);
            std::process::exit(1);
        }
    }
}
