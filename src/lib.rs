use std::{collections::HashMap, fs, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    root: String,
    workspaces: HashMap<String, Workspace>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Workspace {
    projects: HashMap<String, Project>,     // make optional
    workspaces: HashMap<String, Workspace>, // make optional
}

#[derive(Debug, Clone, Deserialize)]
pub struct Project;

impl Config {
    pub fn file_path() -> Result<PathBuf> {
        let home_dir = home::home_dir().expect("Could not determine home directory");
        Ok(home_dir.clone().join(".config/workspaces/workspaces.yaml"))
    }

    pub fn from_config_file() -> Result<Self> {
        let config_file = fs::read_to_string(Self::file_path()?)
            .context("Tried reading ~/.config/workspaces/workspaces.yaml")?;

        Self::from_str(config_file.as_str())
    }

    pub(crate) fn from_str(contents: &str) -> Result<Self> {
        let home_dir = home::home_dir().expect("Could not determine home directory");

        serde_yaml::from_str(contents)
            .context("Tried loading config from ~/.config/workspaces/workspaces.yaml")
            .and_then(|c: Self| {
                if !c.root.starts_with("~") {
                    return Ok(c);
                }
                let mut c = c;
                c.root = home_dir
                    .into_os_string()
                    .into_string()
                    .map_err(|err| anyhow!("Error: {:?}", err))
                    .context("Something unexpected happened")?;
                Ok(c)
            })
    }

    pub fn collect_workspace_paths(&self) -> Vec<PathBuf> {
        let parent = PathBuf::from(self.root.clone());

        self.workspaces
            .iter()
            .map(|(name, ws)| {
                let path = parent.clone().join(name);
                let mut nested = ws.collect_workspace_paths(path.clone());
                nested.push(path);
                nested
            })
            .collect::<Vec<Vec<PathBuf>>>()
            .concat()
    }

    pub fn collect_project_paths(&self) -> Vec<PathBuf> {
        let parent = PathBuf::from(self.root.clone());

        self.workspaces
            .iter()
            .map(|(name, ws)| {
                let path = parent.clone().join(name);
                ws.collect_project_paths(path.clone())
            })
            .collect::<Vec<Vec<PathBuf>>>()
            .concat()
    }
}

impl Workspace {
    pub fn collect_workspace_paths(&self, parent: PathBuf) -> Vec<PathBuf> {
        self.workspaces
            .iter()
            .map(|(name, ws)| {
                let path = parent.clone().join(name);
                let mut nested = ws.collect_workspace_paths(path.clone());
                nested.push(path);
                nested
            })
            .collect::<Vec<Vec<PathBuf>>>()
            .concat()
    }

    pub fn collect_project_paths(&self, parent: PathBuf) -> Vec<PathBuf> {
        let projects = self
            .projects
            .iter()
            .map(|(name, _)| parent.clone().join(name))
            .collect::<Vec<PathBuf>>();

        let nested_projects = self
            .workspaces
            .iter()
            .map(|(name, ws)| {
                let path = parent.clone().join(name);
                ws.collect_project_paths(path.clone())
            })
            .collect::<Vec<Vec<PathBuf>>>()
            .concat();

        vec![projects, nested_projects].concat()
    }
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
