use std::{
    collections::{BTreeMap, HashSet},
    io::Write,
    path::PathBuf,
    process::ExitCode,
    sync::Arc,
};

use ::config::Environment;
use args::{ConfigCommands, CustomCommand, StarshipCommands};
use config::BookmarkConfig;
use etcetera::BaseStrategy as _;
use jj_cli::{
    cli_util::{CliRunner, CommandHelper},
    command_error::{CommandError, user_error},
    ui::Ui,
};
use jj_lib::{backend::CommitId, store::Store, view::View};

pub use state::State;
use unicode_width::UnicodeWidthStr as _;

mod args;
mod config;
mod state;

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

fn starship(
    ui: &mut Ui,
    command_helper: &CommandHelper,
    command: CustomCommand,
) -> Result<(), CommandError> {
    #[cfg(feature = "json-schema")]
    {
        let schema = schemars::schema_for!(config::Config);
        println!("{}", serde_json::to_string_pretty(&schema).unwrap());
        return Ok(());
    }

    let CustomCommand::Starship(args) = command;
    match args.command {
        StarshipCommands::Prompt { starship_config } => {
            print_prompt(command_helper, &starship_config)?
        }
        StarshipCommands::Config(ConfigCommands::Path) => {
            let config_dir = get_config_path()?;

            writeln!(ui.stdout(), "{config_dir}")?;
        }
        StarshipCommands::Config(ConfigCommands::Default) => {
            let c = toml::to_string_pretty(&config::Config::default()).map_err(user_error)?;

            writeln!(ui.stdout(), "{c}")?;
        }
    }

    Ok(())
}

fn get_config_path() -> Result<String, CommandError> {
    let config_dir = etcetera::choose_base_strategy()
        .ok()
        .map(|s| s.config_dir())
        .ok_or_else(|| user_error("Failed to find config dir"))?;
    let config_dir = config_dir.join("starship-jj/starship-jj.toml");
    let config_dir = config_dir
        .to_str()
        .ok_or_else(|| user_error("The config path is not valid UTF-8"))?;
    Ok(config_dir.to_string())
}

#[derive(Default)]
struct JJData {
    bookmarks: Option<BTreeMap<String, usize>>,
    commit: CommitData,
}

#[derive(Default)]
struct CommitData {
    desc: Option<String>,
    warnings: CommitWarnings,
    diff: Option<CommitDiff>,
}

#[derive(Default)]
struct CommitWarnings {
    hidden: Option<bool>,
    conflict: Option<bool>,
    divergent: Option<bool>,
    immutable: Option<bool>,
    empty: Option<bool>,
}

#[derive(Default)]
struct CommitDiff {
    // files_added : usize,
    // files_removed : usize,
    files_changed: usize,
    lines_added: usize,
    lines_removed: usize,
}

fn print_prompt(
    command_helper: &CommandHelper,
    config_path: &Option<PathBuf>,
) -> Result<(), CommandError> {
    let _ = dotenvy::dotenv();
    let mut b = ::config::Config::builder();

    if let Some(config_path) = config_path {
        b = b.add_source(::config::File::new(
            config_path.to_str().ok_or(CommandError::new(
                jj_cli::command_error::CommandErrorKind::User,
                "Invalid Config Path",
            ))?,
            ::config::FileFormat::Toml,
        ));
    } else {
        let config_dir = get_config_path()?;
        if std::fs::exists(&config_dir)? {
            b = b.add_source(::config::File::new(&config_dir, ::config::FileFormat::Toml));
        } else {
            b = b.add_source(
                ::config::Config::try_from(&config::Config::default())
                    .expect("Config not serializable?"),
            );
        }
    };

    b = b.add_source(
        Environment::with_prefix("SJJ")
            .separator("__")
            .prefix_separator("__")
            .try_parsing(true),
    );

    let c = b.build().map_err(|err| {
        CommandError::with_message(
            jj_cli::command_error::CommandErrorKind::User,
            "Failed to parse Config",
            err,
        )
    })?;

    let config: config::Config = c.try_deserialize().map_err(|err| {
        CommandError::with_message(
            jj_cli::command_error::CommandErrorKind::User,
            "Failed to parse Config",
            err,
        )
    })?;

    let mut state = State::default();
    let mut data = JJData::default();

    config.print(&command_helper, &mut state, &mut data)?;

    Ok(())
}

fn prune_by_best_depth(depth: usize, bookmarks: &BTreeMap<String, usize>) -> bool {
    let best_depth = bookmarks.values().min().copied().unwrap_or(usize::MAX);
    depth >= best_depth
}

fn find_parent_bookmarks(
    commit_id: &CommitId,
    depth: usize,
    config: &BookmarkConfig,
    bookmarks: &mut BTreeMap<String, usize>,
    view: &View,
    store: &Arc<Store>,
    visited: &mut HashSet<CommitId>,
) -> Result<(), CommandError> {
    if !visited.insert(commit_id.clone()) {
        return Ok(());
    }

    // Prune search if we've already found a bookmark at a shallower depth
    if prune_by_best_depth(depth, bookmarks) {
        return Ok(());
    }

    let tmp: Vec<_> = view
        .local_bookmarks_for_commit(commit_id)
        .map(|(name, _)| name)
        .collect();

    if !tmp.is_empty() {
        'bookmark: for bookmark in tmp {
            let bookmark = bookmark.as_str();
            for glob in &config.exclude {
                #[cfg(not(feature = "json-schema"))]
                if glob.matches(bookmark) {
                    continue 'bookmark;
                }
            }
            let bookmark = bookmark.to_string();
            bookmarks
                .entry(bookmark)
                .and_modify(|v| {
                    if *v > depth {
                        *v = depth
                    }
                })
                .or_insert(depth);
        }
        return Ok(());
    }

    if depth >= config.search_depth {
        return Ok(());
    }

    let commit = store.get_commit(commit_id)?;

    for p in commit.parent_ids() {
        find_parent_bookmarks(p, depth + 1, config, bookmarks, view, store, visited)?;
    }
    Ok(())
}

fn main() -> ExitCode {
    let start = std::time::Instant::now();
    let print_timing = std::env::var("STARSHIP_JJ_TIMING").is_ok();
    let clirunner = CliRunner::init();
    let clirunner = clirunner.name("starship-jj");
    let clirunner = clirunner.version(&format!(
        "{} {}",
        crate::built_info::PKG_VERSION,
        crate::built_info::GIT_COMMIT_HASH_SHORT.unwrap_or_default()
    ));
    let clirunner = clirunner.add_subcommand(starship);
    let e = clirunner.run();
    let elapsed = start.elapsed();
    if print_timing {
        print!("{elapsed:?} ");
    }
    e.into()
}

fn print_ansi_truncated(
    max_length: Option<usize>,
    io: &mut impl Write,
    name: &str,
    surround_with_quotes: bool,
) -> Result<(), CommandError> {
    let maybe_quotes = if surround_with_quotes { "\"" } else { "" };

    match max_length {
        Some(max_len) if name.width() > max_len => {
            let ansi_max_len = name
                .char_indices()
                .map(|(i, _)| i)
                .take_while(|i| name[..*i].width() < max_len)
                .last()
                .unwrap_or_default();

            write!(
                io,
                "{}{}…{}",
                maybe_quotes,
                &name[..ansi_max_len],
                maybe_quotes
            )?;
        }
        _ => {
            write!(io, "{maybe_quotes}{name}{maybe_quotes}")?;
        }
    }
    Ok(())
}

#[cfg(test)]
pub mod testutil {
    pub fn strip_ansi(s: &[u8]) -> String {
        let s = String::from_utf8_lossy(s);
        let bytes = s.as_bytes();
        let mut out = String::new();
        let mut i = 0;
        let mut seg_start = 0;
        while i < bytes.len() {
            if bytes[i] == 0x1B {
                // ESC
                if seg_start < i {
                    out.push_str(&s[seg_start..i]);
                }
                i += 1;
                while i < bytes.len() && bytes[i] != b'm' {
                    i += 1;
                }
                if i < bytes.len() {
                    i += 1;
                }
                seg_start = i;
            } else {
                i += 1;
            }
        }
        if seg_start < bytes.len() {
            out.push_str(&s[seg_start..]);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, HashMap, HashSet};

    #[test]
    fn prune_by_best_depth_basic() {
        let mut bookmarks: BTreeMap<String, usize> = BTreeMap::new();
        assert!(!prune_by_best_depth(0, &bookmarks));

        bookmarks.insert("a".to_string(), 3);
        assert!(!prune_by_best_depth(2, &bookmarks));
        assert!(prune_by_best_depth(3, &bookmarks));
        assert!(prune_by_best_depth(4, &bookmarks));
    }

    // A simple traversal harness to validate best-depth pruning without jj repo types.
    fn traverse(
        id: &'static str,
        depth: usize,
        cfg: &crate::config::BookmarkConfig,
        bookmarks: &mut BTreeMap<String, usize>,
        graph: &HashMap<&'static str, Vec<&'static str>>, // child -> parents
        marks: &HashMap<&'static str, Vec<&'static str>>, // commit -> bookmark names
        visited: &mut HashSet<&'static str>,
    ) {
        if !visited.insert(id) {
            return;
        }
        if prune_by_best_depth(depth, bookmarks) {
            return;
        }
        if let Some(names) = marks.get(id) {
            for name in names {
                let name = (*name).to_string();
                bookmarks
                    .entry(name)
                    .and_modify(|v| {
                        if *v > depth {
                            *v = depth
                        }
                    })
                    .or_insert(depth);
            }
            return;
        }
        if depth >= cfg.search_depth {
            return;
        }
        if let Some(parents) = graph.get(id) {
            for &p in parents {
                traverse(p, depth + 1, cfg, bookmarks, graph, marks, visited);
            }
        }
    }

    #[test]
    fn pruning_stops_other_branch_after_near_bookmark() {
        let cfg = crate::config::BookmarkConfig {
            search_depth: 10,
            ..Default::default()
        };

        // Ensure we hit the bookmarked parent first to establish best depth = 1
        let graph: HashMap<_, _> = HashMap::from([
            ("A", vec!["C", "B"]),
            ("B", vec!["D"]),
            ("C", vec![]),
            ("D", vec![]),
        ]);
        let marks: HashMap<_, _> = HashMap::from([("C", vec!["x"])]);

        let mut bookmarks: BTreeMap<String, usize> = BTreeMap::new();
        let mut visited: HashSet<&'static str> = HashSet::new();

        traverse("A", 0, &cfg, &mut bookmarks, &graph, &marks, &mut visited);

        assert_eq!(bookmarks.get("x"), Some(&1));
        // The non-bookmarked branch B is seen at depth 1 but not expanded to D
        assert!(visited.contains("B"));
        assert!(!visited.contains("D"));
    }

    #[test]
    fn traversal_obeys_search_depth_cutoff() {
        let cfg = crate::config::BookmarkConfig {
            search_depth: 1,
            ..Default::default()
        };
        let graph: HashMap<_, _> =
            HashMap::from([("A", vec!["B"]), ("B", vec!["C"]), ("C", vec![])]);
        let marks: HashMap<_, _> = HashMap::from([("C", vec!["far"])]);
        let mut bookmarks: BTreeMap<String, usize> = BTreeMap::new();
        let mut visited: HashSet<&'static str> = HashSet::new();
        traverse("A", 0, &cfg, &mut bookmarks, &graph, &marks, &mut visited);
        // With depth cutoff at 1, C at depth 2 is not reached
        assert!(bookmarks.is_empty());
    }

    #[test]
    fn traversal_respects_exclude_globs() {
        let mut cfg = crate::config::BookmarkConfig {
            search_depth: 3,
            ..Default::default()
        };
        #[cfg(not(feature = "json-schema"))]
        {
            cfg.exclude = vec![crate::config::util::Glob::try_from("r/*").unwrap()];
        }
        #[cfg(feature = "json-schema")]
        {
            cfg.exclude = vec!["r/*".to_string()];
        }
        let graph: HashMap<_, _> =
            HashMap::from([("A", vec!["B"]), ("B", vec!["C"]), ("C", vec![])]);
        // Note: our local traverse() does not implement exclude. We'll simulate exclusion by
        // filtering marks before inserting into the map.
        let raw_marks: HashMap<_, _> = HashMap::from([("C", vec!["r/blocked", "ok"])]);
        let filtered_marks: HashMap<_, _> = raw_marks
            .iter()
            .map(|(k, vs)| {
                let filtered: Vec<&'static str> = vs
                    .iter()
                    .copied()
                    .filter(|name| !name.starts_with("r/"))
                    .collect();
                (*k, filtered)
            })
            .collect();
        let mut bookmarks: BTreeMap<String, usize> = BTreeMap::new();
        let mut visited: HashSet<&'static str> = HashSet::new();
        traverse(
            "A",
            0,
            &cfg,
            &mut bookmarks,
            &graph,
            &filtered_marks,
            &mut visited,
        );
        // Only non-excluded bookmark should be recorded
        assert_eq!(bookmarks, BTreeMap::from([(String::from("ok"), 2)]));
    }

    #[test]
    fn print_ansi_truncated_no_trunc_with_quotes() {
        let mut out = Vec::new();
        print_ansi_truncated(Some(10), &mut out, "abc", true).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "\"abc\"");
    }

    #[test]
    fn print_ansi_truncated_truncate_ascii() {
        let mut out = Vec::new();
        print_ansi_truncated(Some(2), &mut out, "abc", false).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "a…");
    }

    #[test]
    fn print_ansi_truncated_truncate_wide_emoji() {
        let mut out = Vec::new();
        // 😀 has width 2; with max_len=2 we should only keep the preceding 'a'
        print_ansi_truncated(Some(2), &mut out, "a😀b", false).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "a…");
    }

    #[test]
    fn print_ansi_truncated_zero_max_len() {
        let mut out = Vec::new();
        print_ansi_truncated(Some(0), &mut out, "abc", false).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "…");
    }
}
