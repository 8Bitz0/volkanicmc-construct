use clap::{Parser, Subcommand};
use std::path;
use tracing::{info, error};

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
    },
    Check { path: path::PathBuf },
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
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    match args.command {
        Command::Build { path, force } => {
            let template = match template::Template::import(path::PathBuf::from(path))
                .await {
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

            match build::build(template, store, force).await {
                Ok(()) => {},
                Err(e) => {
                    error!("Failed to build template: {}", e);
                    std::process::exit(1);
                }
            };
        }
        Command::Check { path } => {
            let template = match template::Template::import(path::PathBuf::from(path))
                .await {
                    Ok(template) => template,
                    Err(e) => {
                        error!("Failed to parse template: {}", e);
                        std::process::exit(1);
                    }
            };
            info!("Template \"{}\" parsed correctly", template.name);
        },
        Command::Create => {
            println!("{}", match serde_jsonc::to_string_pretty(&template::Template::default()) {
                Ok(json) => json,
                Err(e) => {
                    error!("Failed to serialize template: {}", e);
                    std::process::exit(1);
                }
            });
        },
        Command::Template(command) => match command {
            TemplateCommand::Embed { path } => {
                let template = match template::Template::import(path)
                    .await {
                        Ok(template) => template,
                        Err(e) => {
                            error!("Failed to parse template: {}", e);
                            std::process::exit(1);
                        }
                };

                match template::manage::embed(template).await {
                    Ok(t) => {
                        println!("{}", match serde_jsonc::to_string_pretty(&t) {
                            Ok(json) => json,
                            Err(e) => {
                                error!("Failed to serialize template: {}", e);
                                std::process::exit(1);
                            }
                        });
                    },
                    Err(e) => {
                        error!("Failed to embed template: {}", e);
                        std::process::exit(1);
                    }
                };
            }
        }
    }
}
