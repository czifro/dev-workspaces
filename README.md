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
# $HOME/.config/workspaces/workspaces.yaml
root: ~/
workspaces:
  src:
    workspaces:
      nested:
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

# CLI Usage

```shell
$ workspaces help
A dev tool to simplify working with workspace directories

Usage: workspaces <COMMAND>

Commands:
  list    List out managed paths
  doctor  Show doctor diagnosis on managed workspaces and projects
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

$ workspaces help list
List out managed paths

Usage: workspaces list <COMMAND>

Commands:
  workspaces  List workspace paths
  projects    List project paths
  help        Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

$ workspaces list help workspaces
List workspace paths

Usage: workspaces list workspaces

Options:
  -h, --help     Print help
  -V, --version  Print version

$ workspaces list help projects
List project paths

Usage: workspaces list projects

Options:
  -h, --help     Print help
  -V, --version  Print version

$ workspaces help doctor
Show doctor diagnosis on managed workspaces and projects

Usage: workspaces doctor

Options:
  -h, --help     Print help
  -V, --version  Print version

$ workspaces help config
Show config path

Usage: workspaces config [OPTIONS]

Options:
  -q, --quiet    Quiet extraneous output
  -h, --help     Print help
  -V, --version  Print version

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


