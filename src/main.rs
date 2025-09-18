use std::{collections::BTreeMap, io::Write, path::PathBuf, process::ExitCode};

use args::{ConfigCommands, CustomCommand, StarshipCommands};
// use config::BookmarkConfig;
use etcetera::BaseStrategy as _;
use jj_cli::{
    cli_util::{CliRunner, CommandHelper},
    command_error::{CommandError, user_error},
    ui::Ui,
};
// use jj_lib::{backend::CommitId, store::Store, view::View};

pub use state::State;
use unicode_width::UnicodeWidthStr as _;

mod args;
mod config;
mod search;
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
    // Load .env only when opted-in via feature and dev/explicit request
    #[cfg(feature = "dotenv")]
    {
        if cfg!(debug_assertions) || std::env::var_os("STARSHIP_JJ_USE_DOTENV").is_some() {
            let _ = dotenvy::dotenv();
        }
    }

    let mut config = load_config_file(config_path)?;
    config.apply_env_overrides_from_env()?;

    let mut state = State::default();
    let mut data = JJData::default();

    config.print(&command_helper, &mut state, &mut data)?;

    Ok(())
}

fn load_config_file(config_path: &Option<PathBuf>) -> Result<config::Config, CommandError> {
    use std::io::Read as _;

    let p: PathBuf = if let Some(p) = config_path {
        p.clone()
    } else {
        PathBuf::from(get_config_path()?)
    };

    if std::fs::exists(&p)? {
        let mut s = String::new();
        let mut f = std::fs::File::open(&p)?;
        f.read_to_string(&mut s)?;
        Ok(toml::from_str::<config::Config>(&s).map_err(user_error)?)
    } else {
        Ok(config::Config::default())
    }
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
    use unicode_width::UnicodeWidthChar as _;

    let maybe_quotes = if surround_with_quotes { "\"" } else { "" };

    match max_length {
        Some(max_len) => {
            // Only truncate if total display width exceeds max_len
            if name.width() <= max_len {
                write!(io, "{maybe_quotes}{name}{maybe_quotes}")?;
                return Ok(());
            }
            // Fast path: zero max length always truncates to just the ellipsis
            if max_len == 0 {
                write!(io, "{}…{}", maybe_quotes, maybe_quotes)?;
                return Ok(());
            }

            // Walk characters once, keeping last byte index where prefix width < max_len
            let mut acc_width = 0usize;
            let mut cut_byte = 0usize; // prefix start
            for (i, ch) in name.char_indices() {
                if acc_width < max_len {
                    cut_byte = i; // prefix up to i has width < max_len
                }
                acc_width += ch.width().unwrap_or(0);
                if acc_width >= max_len {
                    break;
                }
            }

            if cut_byte < name.len() {
                write!(io, "{}{}…{}", maybe_quotes, &name[..cut_byte], maybe_quotes)?;
            } else {
                write!(io, "{maybe_quotes}{name}{maybe_quotes}")?;
            }
        }
        None => {
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

    #[test]
    fn get_config_path_suffix_and_utf8() {
        let p = get_config_path().unwrap();
        assert!(p.ends_with("starship-jj/starship-jj.toml") || p.ends_with("starship-jj.toml"));
        // String already implies valid UTF-8; also ensure not empty
        assert!(!p.is_empty());
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
