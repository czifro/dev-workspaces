use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};

use dev_workspaces::*;

#[derive(Parser)]
#[command(name = "workspaces")]
#[command(bin_name = "workspaces")]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List out managed paths
    #[command(subcommand)]
    List(ListCommand),

    /// Show doctor diagnosis on managed workspaces and projects
    Doctor,

    /// Restore workspaces and projects
    #[command(subcommand)]
    Restore(RestoreCommand),

    /// Show config path
    Config {
        /// Quiet extraneous output
        #[arg(short, long)]
        quiet: bool,
    },
}

#[derive(Subcommand)]
enum ListCommand {
    /// List workspace paths
    Workspaces,

    /// List project paths
    Projects,
}

#[derive(Subcommand)]
enum RestoreCommand {
    #[command(long_about = Some(r#"
Restore a workspsce by relative path or all workspaces with the option to include projects

Examples:
   workspaces restore workspace path/of/workspace
   workspaces restore workspace path/of/workspace --include-projects
   workspaces restore workspace --all
"#))]
    Workspace {
        /// Restore a workspace by path
        path: Option<String>,
        /// Restore projects in the workspace
        #[arg(long)]
        include_projects: bool,
        /// Restore all workspaces
        #[arg(long)]
        all: bool,
    },
    #[command(long_about = Some(r#"
Restore a project by relative path

Example:
   workspaces restore project path/of/workspace/project
"#))]
    Project(RestoreProjectCommand),
}

#[derive(Args)]
struct RestoreProjectCommand {
    /// Restore a project by path
    path: String,
}

fn main() -> Result<()> {
    let config = Config::from_config_file()?;

    let workspace_paths = config.collect_workspace_paths();

    let project_paths = config.collect_project_paths();

    let cli = Cli::parse();

    match &cli.command {
        Commands::List(cmd) => {
            match &cmd {
                ListCommand::Workspaces => {
                    for p in workspace_paths.iter() {
                        let p = <PathBuf as Clone>::clone(p)
                            .into_os_string()
                            .into_string()
                            .unwrap();
                        println!("{p}");
                    }
                }
                ListCommand::Projects => {
                    for p in project_paths.iter() {
                        let p = <PathBuf as Clone>::clone(p)
                            .into_os_string()
                            .into_string()
                            .unwrap();
                        println!("{p}");
                    }
                }
            };
        }
        Commands::Doctor { .. } => {
            let diagnosis = doctor(&config).context("Tried to generate doctor diagnosis")?;
            diagnosis.print();
        }
        Commands::Config { quiet } => {
            let config_path = Config::file_path()?;
            let config_path = config_path.into_os_string().into_string().unwrap();
            if *quiet {
                println!("{config_path}");
            } else {
                println!("Workspaces config path: {config_path}");
            }
        }
        Commands::Restore(cmd) => {
            match &cmd {
                RestoreCommand::Workspace {
                    path,
                    include_projects,
                    all,
                } => {
                    if *all {
                        return restore(
                            &config,
                            RestoreOption::AllWorkspaces {
                                include_projects: *include_projects,
                            },
                        )
                        .context("Failed to restore all");
                    }
                    let path = path
                        .clone()
                        .ok_or_else(|| anyhow::anyhow!("Workspace path is required"))?;
                    restore(
                        &config,
                        RestoreOption::Workspace {
                            ws_path: PathBuf::from(path),
                            include_projects: *include_projects,
                        },
                    )
                    .context("Failed to restore workspace")?;
                }
                RestoreCommand::Project(RestoreProjectCommand { path }) => {
                    restore(
                        &config,
                        RestoreOption::Project {
                            proj_path: PathBuf::from(path),
                        },
                    ).context("Failed to restore project")?;
                },
            };
        }
    };

    Ok(())
}
