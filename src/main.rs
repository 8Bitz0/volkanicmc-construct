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
    Build {
        path: path::PathBuf,
        #[arg(short = 'f', long)]
        force: bool,
        #[arg(short = 'v', long)]
        user_vars: Vec<String>,
    },
    Check {
        path: path::PathBuf,
    },
    Run,
    #[command(subcommand)]
    Template(TemplateCommand),
    Create,
}

#[derive(Debug, Clone, Subcommand)]
enum TemplateCommand {
    /// Moves all external "include" files into template
    Embed { path: path::PathBuf },
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

            match build::build(template, store, force, user_vars).await {
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
        Command::Create => {
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
        },
    }
}
