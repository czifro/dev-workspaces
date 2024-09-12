use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
};

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

use crate::git::{GitCloneProtocol, GitCloneStrategy, GitHost};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub(crate) root: String,
    pub(crate) git: GitConfig,
    pub(crate) workspaces: HashMap<String, Workspace>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitConfig {
    pub(crate) clone_strategy: Option<GitCloneStrategy>,
    pub(crate) protocol: Option<GitCloneProtocol>,
    pub(crate) host: Option<GitHost>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Workspace {
    pub(crate) projects: HashMap<String, Project>,
    pub(crate) git: Option<GitConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Project {
    pub(crate) git: Option<ProjectGitSettings>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectGitSettings {
    pub(crate) repo: String,
    #[serde(flatten)]
    pub(crate) core_settings: GitConfig,
}

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
        serde_yaml::from_str(contents)
            .context("Tried loading config from ~/.config/workspaces/workspaces.yaml")
            .and_then(|c: Self| {
                let mut c = c;
                c.root = super::absolute_path(c.root);

                for ws in c.workspaces.values_mut() {
                    ws.overlay_git_config(c.git.clone());
                }

                Ok(c)
            })
    }

    pub fn collect_workspace_paths(&self) -> Vec<PathBuf> {
        let parent = PathBuf::from(self.root.clone());

        self.workspaces
            .iter()
            .map(|(name, _ws)| {
                let path = parent.clone().join(name);
                path
            })
            .collect::<Vec<PathBuf>>()
    }

    pub fn collect_project_paths(&self) -> Vec<PathBuf> {
        let parent = PathBuf::from(self.root.clone());

        self.workspaces
            .iter()
            .map(|(name, ws)| {
                let path = parent.clone().join(name);
                ws.collect_project_paths(&path)
            })
            .collect::<Vec<Vec<PathBuf>>>()
            .concat()
    }

    pub(crate) fn lookup_workspace(&self, ws_path: &PathBuf) -> Result<&Workspace> {
        let mut ws_path = ws_path.clone();
        if ws_path.starts_with(&self.root) {
            ws_path = ws_path.strip_prefix(&self.root).unwrap().to_path_buf();
        }
        let ws_path = ws_path;

        let ws = self
            .workspaces
            .get(&ws_path.clone().into_os_string().into_string().unwrap());
        let Some(workspace) = ws else {
            return Err(anyhow!(
                "Could not find workspace: {:}",
                ws_path.clone().into_os_string().into_string().unwrap()
            ));
        };

        Ok(workspace)
    }

    pub(crate) fn lookup_project(&self, proj_path: &PathBuf) -> Result<&Project> {
        let Some(ws_path) = proj_path.parent() else {
            return Err(anyhow!("Expected project path to be sub path to workspace"));
        };
        let ws_path = &ws_path.to_path_buf();
        let proj_name = proj_path.strip_prefix(ws_path).unwrap().to_path_buf();
        let workspace = self.lookup_workspace(ws_path)?;

        let Some(project) = workspace
            .projects
            .get(&proj_name.into_os_string().into_string().unwrap())
        else {
            return Err(anyhow!(
                "Could not find project: {:}",
                proj_path.clone().into_os_string().into_string().unwrap()
            ));
        };

        Ok(project)
    }
}

impl Workspace {
    pub(crate) fn collect_project_paths(&self, parent: &PathBuf) -> Vec<PathBuf> {
        self.projects
            .iter()
            .map(|(name, _)| parent.clone().join(name))
            .collect::<Vec<PathBuf>>()
    }

    pub(crate) fn overlay_git_config(&mut self, g: GitConfig) {
        let Some(mut ws_git) = self.git.clone().or(Some(g.clone())) else {
            return;
        };

        ws_git.host = ws_git.host.or(g.host);
        ws_git.protocol = ws_git.protocol.or(g.protocol);
        ws_git.clone_strategy = ws_git.clone_strategy.or(g.clone_strategy);

        for p in self.projects.values_mut() {
            p.overlay_git_config(ws_git.clone());
        }

        self.git = Some(ws_git.clone());
    }
}

impl Project {
    pub(crate) fn overlay_git_config(&mut self, g: GitConfig) {
        let Some(mut proj_git) = self.git.clone() else {
            return;
        };

        proj_git.core_settings.host = proj_git.core_settings.host.or(g.host);
        proj_git.core_settings.protocol = proj_git.core_settings.protocol.or(g.protocol);
        proj_git.core_settings.clone_strategy =
            proj_git.core_settings.clone_strategy.or(g.clone_strategy);

        self.git = Some(proj_git);
    }
}

