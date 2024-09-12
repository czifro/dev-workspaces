use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::ProjectGitSettings;

pub(crate) struct Git {
    path: PathBuf,
    repo: String,
    host: GitHost,
    clone_options: GitCloneOptions,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GitHost {
    GitHub,
    GitLab,
}

pub(crate) struct GitCloneOptions {
    strategy: GitCloneStrategy,
    protocol: GitCloneProtocol,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitCloneStrategy {
    Worktree,
    Branch,
}

impl GitCloneStrategy {
    pub(super) fn is_worktree(&self) -> bool {
        match self {
            Self::Worktree { .. } => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GitCloneProtocol {
    HTTPS,
    SSH,
}

impl Git {
    pub(crate) fn new(path: PathBuf, proj_git: ProjectGitSettings) -> Git {
        Self {
            path,
            repo: proj_git.repo,
            host: proj_git.core_settings.host.unwrap_or(GitHost::GitHub),
            clone_options: GitCloneOptions {
                strategy: proj_git
                    .core_settings
                    .clone_strategy
                    .unwrap_or(GitCloneStrategy::Branch),
                protocol: proj_git
                    .core_settings
                    .protocol
                    .unwrap_or(GitCloneProtocol::HTTPS),
            },
        }
    }

    pub(crate) fn clone(&mut self) -> Result<()> {
        if self.path.exists() {
            return Ok(());
        }

        let git_config = git2::Config::new().context("Tried loading git config")?;

        let path = self.path.clone();
        let mut path = path;
        if self.clone_options.strategy.is_worktree() {
            fs::create_dir(self.path.clone()).context("Tried restoring project")?;
            path = path.join(".bare");
        }
        let path = path;
        let mut progress = progress::Progress::new("Fetch");

        self.with_creds(&git_config, |url, f| {
            let mut last_update = Instant::now();
            let mut rcb = git2::RemoteCallbacks::new();
            let mut counter = progress::MetricsCounter::<10>::new(0, last_update);
            rcb.credentials(f);
            rcb.transfer_progress(|stats| {
                let indexed_deltas = stats.indexed_deltas();
                let msg = if indexed_deltas > 0 {
                    format!(
                        ", ({}/{}) resolvings deltas",
                        indexed_deltas,
                        stats.total_deltas(),
                    )
                } else {
                    // Receiving objects.
                    //
                    // # Caveat
                    //
                    // Progress bar relies on git2 calling `transfer_progress`
                    // to update its transfer rate, but we cannot guarantee a
                    // periodic call of that callback. Thus if we don't receive
                    // any data for, say, 10 seconds, the rate will get stuck
                    // and never go down to 0B/s.
                    // In the future, we need to find away to update the rate
                    // even when the callback is not called.
                    let now = Instant::now();
                    // Scrape a `received_bytes` to the counter every 300ms.
                    if now - last_update > Duration::from_millis(300) {
                        counter.add(stats.received_bytes(), now);
                        last_update = now;
                    }
                    let (rate, unit) = Self::human_readable_bytes(counter.rate() as u64);
                    format!(", {:.2}{}/s", rate, unit)
                };
                progress
                    .tick(stats.indexed_objects(), stats.total_objects(), &msg)
                    .is_ok()
            });

            let mut opts = git2::FetchOptions::new();
            opts.remote_callbacks(rcb);

            println!("Cloning {}...\r", &url);

            git2::build::RepoBuilder::new()
                .bare(self.clone_options.strategy.is_worktree())
                .fetch_options(opts)
                .clone(url, &path)
                .map(|_| ())
                .context("Tried cloning project")?;

            Ok(())
        })
    }

    /// Formats a number of bytes into a human readable SI-prefixed size.
    /// Returns a tuple of `(quantity, units)`.
    fn human_readable_bytes(bytes: u64) -> (f32, &'static str) {
        static UNITS: [&str; 7] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];
        let bytes = bytes as f32;
        let i = ((bytes.log2() / 10.0) as usize).min(UNITS.len() - 1);
        (bytes / 1024_f32.powi(i as i32), UNITS[i])
    }

    // Based on https://github.com/rust-lang/cargo/blob/5836a96d3c1ca3012a738aa321996c46674a8afc/src/cargo/sources/git/utils.rs#L560
    fn with_creds<F>(&self, git_config: &git2::Config, mut f: F) -> Result<()>
    where
        F: FnMut(&str, &mut git2::Credentials<'_>) -> Result<()>,
    {
        let url = self
            .host
            .to_url(&self.clone_options.protocol, &self.repo, None);
        let url = url.as_str();
        let mut cred_helper = git2::CredentialHelper::new(url);
        cred_helper.config(git_config);

        let mut ssh_username_requested = false;
        let mut cred_helper_bad = false;
        let mut any_attempts = false;
        let mut tried_sshkey = false;

        let orig_url = url;
        let mut res = f(orig_url, &mut |url, username, allowed| {
            any_attempts = true;

            if allowed.contains(git2::CredentialType::USERNAME) {
                debug_assert!(username.is_none());
                ssh_username_requested = true;
                return Err(git2::Error::from_str("gonna try usernames later"));
            }

            if allowed.contains(git2::CredentialType::SSH_KEY) && !tried_sshkey {
                tried_sshkey = true;
                let username = username.unwrap();
                debug_assert!(!ssh_username_requested);
                return git2::Cred::ssh_key_from_agent(username);
            }

            if allowed.contains(git2::CredentialType::USER_PASS_PLAINTEXT) && !cred_helper_bad {
                let r = git2::Cred::credential_helper(git_config, url, username);
                cred_helper_bad = r.is_err();
                return r;
            }

            if allowed.contains(git2::CredentialType::DEFAULT) {
                return git2::Cred::default();
            }

            Err(git2::Error::from_str("no authentication methods succeeded"))
        });

        if ssh_username_requested {
            let mut attempts = vec!["git".to_string(), "Will Czifro".to_string()];
            if let Ok(u) = std::env::var("USER").or_else(|_| std::env::var("USERNAME")) {
                attempts.push(u);
            }
            attempts.reverse();

            while let Some(u) = attempts.pop() {
                let mut attempts = 0;
                let url = self
                    .host
                    .to_url(&self.clone_options.protocol, &self.repo, Some(&u));
                res = f(url.as_str(), &mut |_url, username, allowed| {
                    if allowed.contains(git2::CredentialType::USERNAME) {
                        return git2::Cred::username(&u);
                    }
                    if allowed.contains(git2::CredentialType::SSH_KEY)
                        || allowed.contains(git2::CredentialType::USER_PASS_PLAINTEXT)
                    {
                        debug_assert_eq!(Some(u.as_str()), username);
                        attempts += 1;
                        if attempts == 2 {
                            return git2::Cred::ssh_key_from_agent(username.expect("git username"));
                        }
                    }
                    Err(git2::Error::from_str("no authentication available"))
                });

                if attempts != 2 {
                    break;
                }
            }
        }

        res
    }
}

impl GitHost {
    pub(super) fn to_url(
        &self,
        proto: &GitCloneProtocol,
        repo: &String,
        user: Option<&String>,
    ) -> String {
        match proto {
            GitCloneProtocol::HTTPS => format!("https://{:}/{:}.git", self.to_string(), repo),
            GitCloneProtocol::SSH => format!(
                "{:}@{:}:{:}.git",
                user.unwrap_or(&"git".to_string()),
                self.to_string(),
                repo
            ),
        }
    }
}

impl ToString for GitHost {
    fn to_string(&self) -> String {
        match self {
            Self::GitHub => String::from("github.com"),
            Self::GitLab => String::from("gitlab.com"),
        }
    }
}

mod progress {
    use std::{
        cmp, io::Write, time::{Duration, Instant}
    };

    use anyhow::Result;
    use unicode_width::UnicodeWidthChar;

    use super::shell;

    pub struct Progress {
        state: State,
    }

    struct Throttle {
        first: bool,
        last_update: Instant,
    }

    struct State {
        name: String,
        done: bool,
        throttle: Throttle,
        format: Format,
        last_line: Option<String>,
        shell: shell::Shell,
    }

    struct Format {
        max_width: usize,
        max_print: usize,
    }

    impl Progress {
        pub fn new(name: &str) -> Self {
            let shell = shell::Shell::new();
            Self {
                state: State {
                    name: name.to_string(),
                    format: Format {
                        max_width: shell.err_width().size(80),
                        max_print: 50,
                    },
                    throttle: Throttle::new(),
                    done: false,
                    last_line: None,
                    shell,
                },
            }
        }

        pub fn tick(&mut self, cur: usize, max: usize, msg: &str) -> Result<()> {
            if !self.state.throttle.allowed() {
                return Ok(());
            }

            self.state.tick(cur, max, msg)
        }
    }

    impl Throttle {
        fn new() -> Self {
            Self {
                first: true,
                last_update: Instant::now(),
            }
        }

        fn allowed(&mut self) -> bool {
            if self.first {
                let delay = Duration::from_millis(500);
                if self.last_update.elapsed() < delay {
                    return false;
                }
            } else {
                let interval = Duration::from_millis(100);
                if self.last_update.elapsed() < interval {
                    return false;
                }
            }
            self.update();
            true
        }

        fn update(&mut self) {
            self.first = false;
            self.last_update = Instant::now()
        }
    }

    impl State {
        fn tick(&mut self, cur: usize, max: usize, msg: &str) -> Result<()> {
            if self.done {
                return Ok(());
            }

            if max > 0 && cur == max {
                self.done = true;
            }

            self.try_update_max_width();
            if let Some(pbar) = self.format.progress(cur, max) {
                self.print(&pbar, msg)?;
            }
            Ok(())
        }

        fn print(&mut self, prefix: &str, msg: &str) -> Result<()> {
            self.throttle.update();
            self.try_update_max_width();

            // make sure we have enough room for the header
            if self.format.max_width < 15 {
                return Ok(());
            }

            let mut line = prefix.to_string();
            self.format.render(&mut line, msg);
            while line.len() < self.format.max_width - 15 {
                line.push(' ');
            }

            // Only update if the line has changed.
            let sh = &self.shell;
            if sh.is_cleared() || self.last_line.as_ref() != Some(&line) {
                let sh = &mut self.shell;
                sh.set_needs_clear(false);
                sh.status_header(&self.name)?;
                {
                    let mut stderr = std::io::stderr();
                    let _ = stderr.write_fmt(format_args!("{}\r", line));
                }
                self.last_line = Some(line);
                sh.set_needs_clear(true);
            }

            Ok(())
        }

        fn try_update_max_width(&mut self) {
            self.format.max_width = self.shell.err_width().size(self.format.max_width.clone());
        }
    }

    impl Format {
        fn progress(&self, cur: usize, max: usize) -> Option<String> {
            assert!(cur <= max);

            let pct = (cur as f64) / (max as f64);
            let pct = if !pct.is_finite() { 0.0 } else { pct };
            let stats = format!(" {:6.02}%", pct * 100.0);
            let extra_len = stats.len() + 2 + 15;
            let Some(display_width) = self.width().checked_sub(extra_len) else {
                return None;
            };

            let mut string = String::with_capacity(self.max_width);
            string.push('[');
            let hashes = display_width as f64 * pct;
            let hashes = hashes as usize;

            // Draw the `===>`
            if hashes > 0 {
                for _ in 0..hashes - 1 {
                    string.push('=');
                }
                if cur == max {
                    string.push('=');
                } else {
                    string.push('>');
                }
            }

            // Draw the empty space we have left to do
            for _ in 0..(display_width - hashes) {
                string.push(' ');
            }
            string.push(']');
            string.push_str(&stats);

            Some(string)
        }

        fn render(&self, string: &mut String, msg: &str) {
            let mut avail_msg_len = self.max_width - string.len() - 15;
            let mut ellipsis_pos = 0;
            if avail_msg_len <= 3 {
                return;
            }
            for c in msg.chars() {
                let display_width = c.width().unwrap_or(0);
                if avail_msg_len >= display_width {
                    avail_msg_len -= display_width;
                    string.push(c);
                    if avail_msg_len >= 3 {
                        ellipsis_pos = string.len();
                    }
                } else {
                    string.truncate(ellipsis_pos);
                    string.push_str("...");
                    break;
                }
            }
        }

        fn width(&self) -> usize {
            cmp::min(self.max_width, self.max_print)
        }
    }

    /// A metrics counter storing only latest `N` records.
    pub struct MetricsCounter<const N: usize> {
        /// Slots to store metrics.
        slots: [(usize, Instant); N],
        /// The slot of the oldest record.
        /// Also the next slot to store the new record.
        index: usize,
    }

    impl<const N: usize> MetricsCounter<N> {
        /// Creates a new counter with an initial value.
        pub fn new(init: usize, init_at: Instant) -> Self {
            assert!(N > 0, "number of slots must be greater than zero");
            Self {
                slots: [(init, init_at); N],
                index: 0,
            }
        }

        /// Adds record to the counter.
        pub fn add(&mut self, data: usize, added_at: Instant) {
            self.slots[self.index] = (data, added_at);
            self.index = (self.index + 1) % N;
        }

        /// Calculates per-second average rate of all slots.
        pub fn rate(&self) -> f32 {
            let latest = self.slots[self.index.checked_sub(1).unwrap_or(N - 1)];
            let oldest = self.slots[self.index];
            let duration = (latest.1 - oldest.1).as_secs_f32();
            let avg = (latest.0 - oldest.0) as f32 / duration;
            if f32::is_nan(avg) {
                0f32
            } else {
                avg
            }
        }
    }
}

// Based on https://github.com/rust-lang/cargo/blob/5836a96d3c1ca3012a738aa321996c46674a8afc/src/cargo/core/shell.rs#L588
mod shell {
    use std::{fmt, io::Write};

    use anyhow::Result;
    // use std::borrow::{Borrow, BorrowMut};

    use lazy_static::lazy_static;

    lazy_static! {
        static ref SHELL: Shell = Shell::new();
    }
    //
    // pub fn instance<'a>() -> &'a Shell {
    //     SHELL.borrow()
    // }
    //
    // pub fn instance_mut<'a>() -> &'a mut Shell {
    //     SHELL.borrow_mut()
    // }

    pub struct Shell {
        needs_clear: bool,
        quiet: bool,
    }

    impl Shell {
        pub(super) fn new() -> Self {
            Self {
                needs_clear: false,
                quiet: false,
            }
        }

        pub fn set_needs_clear(&mut self, needs_clear: bool) {
            self.needs_clear = needs_clear;
        }

        pub fn is_cleared(&self) -> bool {
            !self.needs_clear
        }

        fn print(
            &mut self,
            status: &dyn fmt::Display,
            message: Option<&dyn fmt::Display>,
        ) -> Result<()> {
            match self.quiet {
                false => Ok(()),
                _ => {
                    if self.needs_clear {
                        self.err_erase_line();
                    }
                    println!("=={status}==");
                    match message {
                        Some(message) => println!("{message}"),
                        _ => {}
                    };
                    Ok(())
                }
            }
        }

        pub fn status_header<T>(&mut self, status: T) -> Result<()>
        where
            T: fmt::Display,
        {
            self.print(&status, None)
        }

        pub fn err_width(&self) -> TtyWidth {
            imp::err_width()
        }

        pub fn err_erase_line(&mut self) {
            let _ = std::io::stdout().write(b"\x1B[K");
            self.needs_clear = false;
        }
    }

    #[derive(Debug, Clone)]
    pub enum TtyWidth {
        // NoTty,
        Known(usize),
    }

    impl TtyWidth {
        pub fn size(self, _def: usize) -> usize {
            match self {
                // Self::NoTty => def,
                Self::Known(u) => u,
            }
        }
    }

    mod imp {
        use super::*;

        #[cfg(unix)]
        pub fn err_width() -> TtyWidth {
            // TODO: dynamically compute width

            TtyWidth::Known(80)
        }

        #[cfg(windows)]
        pub fn width() -> TtyWidth {
            todo!("Implement windows support for checking shell width")
        }
    }
}
