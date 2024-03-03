use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use workspaces::*;

#[derive(Parser)]
#[command(name = "workspaces")]
#[command(bin_name = "workspaces")]
enum WorkspacesCli {
    #[command(subcommand)]
    List(ListCommand),
}

#[derive(Subcommand)]
enum ListCommand {
    Workspaces,
    Projects,
}

fn main() -> Result<()> {
    let config = Config::from_config_file()?;

    let workspace_paths = config.collect_workspace_paths();

    let project_paths = config.collect_project_paths();

    let cli = WorkspacesCli::parse();

    match &cli {
        WorkspacesCli::List(cmd) => {
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
    };

    Ok(())
}
