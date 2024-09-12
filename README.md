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
    projects:
      project_1:
      project_2:
      project_3:
  src/nested:
    projects:
      project_a:
      project_b:
      project_c:

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

$ workspaces help restore
Restore workspaces and projects

Usage: workspaces restore <COMMAND>

Commands:
  workspace
                 Restore a workspsce by relative path or all workspaces with the option to include projects

                 Examples:
                    workspaces restore workspace path/to/workspace
                    workspaces restore workspace path/to/workspace --include-prokects
                    workspaces restore workspace --all

  project
                 Restore a project by relative path

                 Example:
                    workspaces restore project outer-workspace/inner-workspace/project

  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

$ workspaces restore help workspace
Restore a workspsce by relative path or all workspaces with the option to include projects

Examples:
  workspaces restore workspace path/to/workspace
  workspaces restore workspace path/to/workspace --include-prokects
  workspaces restore workspace --all


Usage: workspaces restore workspace [OPTIONS] [PATH]

Arguments:
  [PATH]
          Restore a workspace by path

Options:
      --include-projects
          Restore projects in the workspace

      --all
          Restore all workspaces

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

$ workspaces restore help project
Restore a project by relative path

Example:
   workspaces restore project path/to/workspace/project


Usage: workspaces restore project <PATH>

Arguments:
  <PATH>
          Restore a project by path

Options:
  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

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

Accidentally nuke a project or workspace, use `workspaces` tool to re-clone the
project, or rebuild a workspace and re-clone all projects in the workspace.

Consider the following config file:

```yaml
# $HOME/.config/workspaces/workspaces.yaml
root: ~/
# Default git configuration for all workspaces with projects that have git.repo set
git:
  host: github # optional, defaults to github, options: [github, gitlab]
  clone_strategy: branch # optional, defaults to branch, options: [branch, worktree]
  protocol: ssh # optional, defaults to https, options: [ssh, https]

workspaces:
  src:
    # Override git config for this workspace
    git:
      host: gitlab
    projects:
      project_1:
        git:
          repo: "owner/repo2"
  src/nested:
    projects:
      project_a: # no repo cloned for this project
      project_b:
        git:
          repo: "owner/repo0"
      project_c:
        git:
          repo: "owner/repo1"
          # Worktree clones repo to `~/src/nested/project_c/.bare`
          # this imposes an opinionated structure to projects using worktrees
          # where each worktree is a subdirectory to the project directory
          # instead of the directory of the bare cloned repo
          clone_strategy: worktree

```

If you do not have any workspaces on your file system (i.e. setting up a new machine),
running the following command will completely restore the workspaces:

```shell
$ workspaces restore workspace --all
```

If you would like to include projects in this operation:

```shell
$ workspaces restore workspace --all --include-projects
```

Alternatively, if you want to restore a single workspace or project, run one of
the following:

```shell
$ workspaces restore workspace src/nested
# or...
$ workspaces restore project src/project_1
```

