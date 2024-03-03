# Dev Workspaces

A simple tool to help manage workspace directories and their projects. 

# Getting Started

Install Methods:

```shell
# From crates.io
cargo install dev-workspaces --bin workspaces

# From github repository
cargo install dev-workspaces --git https://github.com/czifro/dev-workspaces --bin workspaces
```

Example:

```yaml
root: ~/
workspaces:
  src:
    workspaces:
      nested
        workspaces: 
        projects:
          project_a:
          project_b:
          project_c:
    projects:
      project_1:
      project_2:
      project_3:

# Run:
# $ workspaces list workspaces
# /<expanded-home-dir>/src
# /<expanded-home-dir>/src/nested
#
# Run:
# $ workspaces list projects
# /<expanded-home-dir>/src/project_1
# /<expanded-home-dir>/src/project_2
# /<expanded-home-dir>/src/project_3
# /<expanded-home-dir>/src/nested/project_a
# /<expanded-home-dir>/src/nested/project_b
# /<expanded-home-dir>/src/nested/project_c
```

# Use Cases

## Tmux Sessionizer

Use `workspaces` with a tmux session tool to open new tmux sessions directly at the
path of a project:

```shell
if [[ $# -eq 1 ]]; then
  selected=$1
else
  selected=$(workspaces list projects | fzf)
fi

if [[ -z $selected ]]; then
  exit 0
fi

selected_name=$(basename "$selected" | tr . _)
tmux_running=$(pgrep tmux)

if [[ -z $TMUX ]] && [[ -z $tmux_running ]]; then
  tmux new-session -s $selected_name -c $selected
  exit 0
fi

if ! tmux has-session -t=$selected_name 2> /dev/null; then
  tmux new-session -ds $selected_name -c $selected
fi

tmux switch-client -t $selected_name
```

## Restore Git Clones

(Pending Feature)

Accidentally nuke a project or workspace, use `workspaces` tool to re-clone the
project, or rebuild a workspace and re-clone all projects in the workspace.


