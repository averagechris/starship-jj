use std::{
    io::Write,
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};

use bookmarks::Bookmarks;
use commit::Commit;
use jj_cli::command_error::CommandError;
use metrics::Metrics;
#[cfg(feature = "json-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use state::State;
use symbol::Symbol;
use util::Glob;

pub mod util;

mod bookmarks;
mod commit;
mod metrics;
mod state;
mod symbol;

#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
    #[serde(flatten, default)]
    global: GlobalConfig,
    /// Modules that will be rendered.
    #[serde(rename = "module", default = "default_modules")]
    modules: Vec<ModuleConfig>,
}

#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[derive(Deserialize, Serialize, Debug)]
pub struct GlobalConfig {
    /// Text that will be printed between each Module.
    #[serde(default = "default_separator")]
    module_separator: String,
    /// Timeout after wich the process is teminated.
    #[serde(default)]
    timeout: Option<u64>,
    /// Controls the behaviour of the bookmark finding algorythm.
    #[serde(default)]
    pub bookmarks: BookmarkConfig,
    /// Controls whether color gets reset at the end.
    #[serde(default = "default_reset_color")]
    pub reset_color: bool,
}

fn default_separator() -> String {
    " ".to_string()
}

fn default_reset_color() -> bool {
    true
}

fn default_modules() -> Vec<ModuleConfig> {
    vec![
        ModuleConfig::Symbol(Default::default()),
        ModuleConfig::Bookmarks(Default::default()),
        ModuleConfig::Commit(Default::default()),
        ModuleConfig::State(Default::default()),
        ModuleConfig::Metrics(Default::default()),
    ]
}

#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[derive(Deserialize, Serialize, Debug)]
pub struct BookmarkConfig {
    /// Controls how far we are looking back to find bookmarks.
    #[serde(default = "default_search_depth")]
    pub search_depth: usize,
    /// Exclude certain bookmarks from the search (supports globs)
    #[serde(default)]
    #[cfg(feature = "json-schema")]
    pub exclude: Vec<String>,
    #[serde(default)]
    #[cfg(not(feature = "json-schema"))]
    pub exclude: Vec<Glob>,
}

impl Default for BookmarkConfig {
    fn default() -> Self {
        Self {
            search_depth: default_search_depth(),
            exclude: Default::default(),
        }
    }
}

fn default_search_depth() -> usize {
    100
}

impl Config {
    pub fn print(
        &self,
        command_helper: &&jj_cli::cli_util::CommandHelper,
        state: &mut crate::State,
        data: &mut crate::JJData,
    ) -> Result<(), CommandError> {
        let done = Arc::new(AtomicBool::new(false));

        let done2 = done.clone();
        if let Some(timeout) = self.global.timeout {
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(timeout));
                if !done2.load(std::sync::atomic::Ordering::Relaxed) {
                    _ = util::Style::default().print(&mut std::io::stdout(), None);
                    print!(" ");
                    let _ = std::io::stdout().flush();
                    std::process::exit(0);
                }
            });
        }
        let mut io = std::io::stdout();
        for module in self.modules.iter() {
            match module {
                ModuleConfig::Bookmarks(bookmarks) => {
                    bookmarks.parse(command_helper, state, data, &self.global)?;
                    let mut io = io.lock();
                    bookmarks.print(&mut io, data, &self.global.module_separator)?;
                }
                ModuleConfig::Commit(commit_desc) => {
                    commit_desc.parse(command_helper, state, data, &self.global)?;
                    let mut io = io.lock();
                    commit_desc.print(&mut io, data, &self.global.module_separator)?
                }
                ModuleConfig::State(commit_warnings) => {
                    commit_warnings.parse(command_helper, state, data, &self.global)?;
                    let mut io = io.lock();
                    commit_warnings.print(&mut io, data, &self.global.module_separator)?
                }
                ModuleConfig::Metrics(commit_diff) => {
                    commit_diff.parse(command_helper, state, data, &self.global)?;
                    let mut io = io.lock();
                    commit_diff.print(&mut io, data, &self.global.module_separator)?
                }
                ModuleConfig::Symbol(symbol) => {
                    symbol.parse(command_helper, state, data, &self.global)?;
                    let mut io = io.lock();
                    symbol.print(&mut io, data, &self.global.module_separator)?
                }
            }
        }
        if self.global.reset_color {
            util::Style::default().print(&mut io, None)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::strip_ansi;

    #[test]
    fn config_print_no_output_when_modules_emit_nothing_and_reset_off() {
        // Construct config without Symbol module, since Symbol always prints.
        let cfg = Config {
            global: GlobalConfig {
                module_separator: super::default_separator(),
                timeout: None,
                bookmarks: Default::default(),
                reset_color: false,
            },
            modules: vec![
                ModuleConfig::Bookmarks(Default::default()),
                ModuleConfig::Commit(Default::default()),
                ModuleConfig::State(Default::default()),
                ModuleConfig::Metrics(Default::default()),
            ],
        };

        // Simulate printing modules directly with no data; each should emit nothing (after stripping ANSI).
        let data = crate::JJData {
            bookmarks: Some(std::collections::BTreeMap::new()),
            ..Default::default()
        };
        let module_sep = &cfg.global.module_separator;
        let mut buf = Vec::new();
        if let ModuleConfig::Bookmarks(b) = &cfg.modules[0] {
            b.print(&mut buf, &data, module_sep).unwrap_or(());
        }
        if let ModuleConfig::Commit(c) = &cfg.modules[1] {
            c.print(&mut buf, &data, module_sep).unwrap_or(());
        }
        if let ModuleConfig::State(s) = &cfg.modules[2] {
            s.print(&mut buf, &data, module_sep).unwrap_or(());
        }
        if let ModuleConfig::Metrics(m) = &cfg.modules[3] {
            m.print(&mut buf, &data, module_sep).unwrap_or(());
        }

        assert_eq!(strip_ansi(&buf), "");
    }

    #[test]
    fn default_module_order_and_reset_color() {
        let cfg = Config::default();
        let names: Vec<&str> = cfg
            .modules
            .iter()
            .map(|m| match m {
                ModuleConfig::Symbol(_) => "Symbol",
                ModuleConfig::Bookmarks(_) => "Bookmarks",
                ModuleConfig::Commit(_) => "Commit",
                ModuleConfig::State(_) => "State",
                ModuleConfig::Metrics(_) => "Metrics",
            })
            .collect();
        assert_eq!(names, ["Symbol", "Bookmarks", "Commit", "State", "Metrics"]);
        assert_eq!(cfg.global.module_separator, super::default_separator());
        assert!(cfg.global.reset_color);
    }
}

/// A module that prints some info about the current jj repo
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "type")]
enum ModuleConfig {
    Symbol(Symbol),
    Bookmarks(Bookmarks),
    Commit(Commit),
    State(State),
    Metrics(Metrics),
}

impl Default for Config {
    fn default() -> Self {
        Self {
            global: GlobalConfig {
                timeout: Default::default(),
                module_separator: default_separator(),
                bookmarks: Default::default(),
                reset_color: default_reset_color(),
            },
            modules: default_modules(),
        }
    }
}
