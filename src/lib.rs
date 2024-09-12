use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};

mod config;
mod git;

pub use config::*;
use git::Git;

pub(crate) fn path_buf_to_string(path: PathBuf) -> Result<String> {
    path.into_os_string()
        .into_string()
        .map_err(|os| anyhow!("{:#?}", os))
        .context("Tried converting path to string")
}

pub(crate) fn try_absolute_path(path: String) -> Result<String> {
    let path = PathBuf::from(path);
    let path: PathBuf = match path.strip_prefix("~") {
        Err(_) => path,
        Ok(path) => {
            let home_dir = home::home_dir().unwrap();
            home_dir.join(path)
        }
    };

    path_buf_to_string(path).context("Tried making path absolute")
}

pub(crate) fn absolute_path(path: String) -> String {
    try_absolute_path(path).unwrap()
}

pub enum RestoreOption {
    Workspace {
        ws_path: PathBuf,
        include_projects: bool,
    },
    AllWorkspaces {
        include_projects: bool,
    },
    Project {
        proj_path: PathBuf,
    },
}

pub fn restore(config: &Config, opt: RestoreOption) -> Result<()> {
    let diagnosis = doctor(config)?;

    match opt {
        RestoreOption::Workspace {
            ws_path,
            include_projects,
        } => {
            let ws = config.lookup_workspace(&ws_path)?;

            // match ws_path.parent() {
            //     Some(parent) if parent != Path::new("") => {
            //         restore(
            //             config,
            //             RestoreOption::Workspace {
            //                 ws_path: parent.to_path_buf(),
            //                 include_projects: false,
            //             },
            //         )?;
            //     }
            //     _ => {}
            // }

            let mut ws_path = ws_path;
            if !ws_path.starts_with(&config.root) {
                ws_path = PathBuf::from(&config.root).join(ws_path);
            }
            let ws_path = ws_path;

            if diagnosis.missing_workspaces.contains(&ws_path) {
                fs::create_dir(&ws_path).context("Tried restoring workspace")?;
            }

            if !include_projects {
                return Ok(());
            }

            for project in ws.collect_project_paths(&ws_path).iter() {
                restore_project(&config, &project)?;
            }
        }
        RestoreOption::AllWorkspaces { include_projects } => {
            for ws_path in diagnosis.missing_workspaces.iter() {
                restore(
                    config,
                    RestoreOption::Workspace {
                        ws_path: ws_path.clone(),
                        include_projects,
                    },
                )?;
            }
        }
        RestoreOption::Project { proj_path } => {
            restore(
                config,
                RestoreOption::Workspace {
                    ws_path: proj_path.parent().unwrap().to_path_buf(),
                    include_projects: false,
                },
            )?;

            let mut proj_path = proj_path;
            if !proj_path.starts_with(&config.root) {
                proj_path = PathBuf::from(&config.root).join(proj_path);
            }
            let proj_path = proj_path;

            if !diagnosis.missing_projects.contains(&proj_path) {
                return Ok(());
            }

            restore_project(config, &proj_path)?;
        }
    };

    Ok(())
}

fn restore_project(config: &Config, proj_path: &PathBuf) -> Result<()> {
    if proj_path.exists() {
        return Ok(());
    }
    let project = config.lookup_project(proj_path)?;

    let Some(ref proj_git) = project.git else {
        return fs::create_dir(proj_path).context("Tried creating project directory");
    };

    let mut g = Git::new(proj_path.clone(), proj_git.clone());

    g.clone()
}

pub struct DoctorDiagnosis {
    missing_workspaces: Vec<PathBuf>,
    missing_projects: Vec<PathBuf>,
}

impl DoctorDiagnosis {
    pub fn print(&self) {
        println!("Dev Workspaces Doctor Diagnosis:\n");

        println!("The following workspaces are missing:\n");

        for w in self.missing_workspaces.iter() {
            println!(
                "\t{:}",
                w.clone()
                    .into_os_string()
                    .into_string()
                    .expect("Something unexpected happened")
            );
        }
        println!("");

        println!("The following projects are missing:\n");

        for p in self.missing_projects.iter() {
            println!(
                "\t{:}",
                p.clone()
                    .into_os_string()
                    .into_string()
                    .expect("Something unexpected happened")
            );
        }
        println!("");
    }
}

pub fn doctor(config: &Config) -> Result<DoctorDiagnosis> {
    let missing_workspaces = config
        .collect_workspace_paths()
        .iter()
        .filter(|p| !p.exists())
        .map(Clone::clone)
        .collect::<Vec<PathBuf>>();
    let missing_projects = config
        .collect_project_paths()
        .iter()
        .filter(|p| !p.exists())
        .map(Clone::clone)
        .collect::<Vec<PathBuf>>();

    Ok(DoctorDiagnosis {
        missing_workspaces,
        missing_projects,
    })
}

#[cfg(test)]
mod should {

    use std::path::PathBuf;

    use rstest::*;

    #[rstest]
    fn list_workspaces() {
        let contents = r#"---
root: /some/root
workspaces:
  w0:
    projects:
      p0:
    workspaces:
      w1:
        projects:
          p1:
        workspaces:
          w2:
            projects:
              p2:
            workspaces:
              w3:
                projects:
                workspaces:
"#;

        let config = super::Config::from_str(contents);

        assert!(config.is_ok());

        let config = config.unwrap();

        let mut workspaces = config.collect_workspace_paths();

        assert_eq!(
            workspaces.sort(),
            vec![
                PathBuf::from("/some/root/w0"),
                PathBuf::from("/some/root/w0/w1"),
                PathBuf::from("/some/root/w0/w1/w2"),
                PathBuf::from("/some/root/w0/w1/w2/w3"),
            ]
            .sort()
        );
    }

    #[rstest]
    fn list_projects() {
        let contents = r#"---
root: /some/root
workspaces:
  w0:
    projects:
      p0:
    workspaces:
      w1:
        projects:
          p1:
        workspaces:
          w2:
            projects:
              p2:
            workspaces:
              w3:
                projects:
                workspaces:
"#;

        let config = super::Config::from_str(contents);

        assert!(config.is_ok());

        let config = config.unwrap();

        let mut projects = config.collect_project_paths();

        assert_eq!(
            projects.sort(),
            vec![
                PathBuf::from("/some/root/w0/p0"),
                PathBuf::from("/some/root/w0/w1/p1"),
                PathBuf::from("/some/root/w0/w1/w2/p2"),
            ]
            .sort()
        );
    }
}
