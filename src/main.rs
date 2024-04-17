use clap::{Parser, Subcommand};
use std::path;
use tracing::{error, info};

mod build;
mod exec;
mod hostinfo;
mod resources;
mod template;
mod vkstore;

#[derive(Debug, Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Build in the current directory using a template from the given path
    Build {
        path: path::PathBuf,
        #[arg(short = 'f', long)]
        force: bool,
        #[arg(short = 'v', long)]
        user_vars: Vec<String>,
        /// Allow custom JVM arguments
        #[arg(long)]
        allow_custom_jvm_args: bool,
        /// Add additional JVM arguments to place before the template's JVM arguments
        #[arg(short = 'j', long, value_parser, num_args = 1.., value_delimiter = ' ')]
        additional_jvm_args: Vec<String>,
    },
    /// Parse a template at the given path
    Check { path: path::PathBuf },
    /// Runs the build in the current directory. Only use for testing with trusted templates. Do not use for execution in production.
    Run,
    #[command(subcommand)]
    Template(TemplateCommand),
    /// Create a Bash script from the execution information of an existing build
    ExecScript,
    /// Clear downloads and temporary files
    Clean,
}

#[derive(Debug, Clone, Subcommand)]
enum TemplateCommand {
    /// Moves all external "include" files into template
    Embed { path: path::PathBuf },
    /// Prints a basic template
    Create,
}

#[tokio::main]
async fn main() {
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
            .with_max_level(tracing::Level::INFO)
            .init();
    }

    let args = Args::parse();

    match args.command {
        Command::Build {
            path,
            force,
            user_vars,
            allow_custom_jvm_args,
            additional_jvm_args,
        } => {
            let template = match template::Template::import(path).await {
                Ok(template) => template,
                Err(e) => {
                    error!("Failed to parse template: {}", e);
                    std::process::exit(1);
                }
            };
            info!("Template \"{}\" parsed correctly", template.name);

            let store = match vkstore::VolkanicStore::init().await {
                Ok(store) => store,
                Err(e) => {
                    error!("Failed to initialize store: {}", e);
                    std::process::exit(1);
                }
            };

            match build::build(
                template,
                store,
                force,
                user_vars,
                allow_custom_jvm_args,
                additional_jvm_args,
            )
            .await
            {
                Ok(()) => {}
                Err(e) => {
                    error!("Failed to build template: {}", e);
                    std::process::exit(1);
                }
            };
        }
        Command::Check { path } => {
            let template = match template::Template::import(path).await {
                Ok(template) => template,
                Err(e) => {
                    error!("Failed to parse template: {}", e);
                    std::process::exit(1);
                }
            };
            info!("Template \"{}\" parsed correctly", template.name);
        }
        Command::Run => {
            let store = match vkstore::VolkanicStore::init().await {
                Ok(store) => store,
                Err(e) => {
                    error!("Failed to initialize store: {}", e);
                    std::process::exit(1);
                }
            };

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
                                    error!("Failed to serialize template: {}", e);
                                    std::process::exit(1);
                                }
                            }
                        );
                    }
                    Err(e) => {
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
                            error!("Failed to serialize template: {}", e);
                            std::process::exit(1);
                        }
                    }
                );
            }
        },
        Command::ExecScript => {
            let store = match vkstore::VolkanicStore::init().await {
                Ok(store) => store,
                Err(e) => {
                    error!("Failed to initialize store: {}", e);
                    std::process::exit(1);
                }
            };

            if !build::BuildInfo::exists(&store).await {
                error!("No build info found");
            }

            let build_info = match build::BuildInfo::get(&store).await {
                Ok(build_info) => build_info,
                Err(e) => {
                    error!("Failed to initialize build info: {}", e);
                    std::process::exit(1);
                }
            };

            let exec_info = match build_info.exec {
                Some(exec_info) => exec_info,
                None => {
                    error!("No execution info provided in build info file");
                    std::process::exit(1);
                }
            };

            println!(
                "{}",
                crate::exec::script::to_script(exec_info, store.build_path)
            );
        }
        Command::Clean => {
            if vkstore::VolkanicStore::exists().await {
                let store = match vkstore::VolkanicStore::init().await {
                    Ok(store) => store,
                    Err(e) => {
                        error!("Failed to initialize store: {}", e);
                        std::process::exit(1);
                    }
                };

                match vkstore::VolkanicStore::clear_downloads(&store).await {
                    Ok(()) => {}
                    Err(e) => {
                        error!("Failed to clean store: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                error!("No store found");
                std::process::exit(1);
            }
        }
    }
}
