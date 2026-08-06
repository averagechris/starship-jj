use std::io::Write;

use futures_util::StreamExt as _;
use jj_cli::{cli_util::RevisionArg, command_error::CommandError, ui::Ui};
use jj_lib::{index::ResolvedChangeState, repo::Repo};
#[cfg(feature = "json-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::util::Style;

/// Prints a warning if the working copy contains any conflicts, is divergent or hidden
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[derive(Deserialize, Serialize, Debug)]
pub struct State {
    /// Text that will be printed between each Warning.
    #[serde(default = "default_separator")]
    separator: String,
    /// Controls how the conflict warning will be rendered.
    #[serde(default = "default_conflict")]
    conflict: Status,
    /// Controls how the divergence warning will be rendered.
    #[serde(default = "default_divergent")]
    divergent: Status,
    /// Controls how the divergence warning will be rendered.
    #[serde(default = "default_empty")]
    empty: Status,
    /// Controls how the empty warning will be rendered.
    #[serde(default = "default_immutable")]
    immutable: Status,
    /// Controls how the immutable warning will be rendered.
    #[serde(default = "default_hidden")]
    hidden: Status,
}

fn default_separator() -> String {
    " ".to_string()
}

fn default_conflict() -> Status {
    Status {
        text: "(CONFLICT)".to_string(),
        style: Style {
            color: Some(super::util::Color::Red),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn default_immutable() -> Status {
    Status {
        text: "(IMMUTABLE)".to_string(),
        style: Style {
            color: Some(super::util::Color::Yellow),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn default_empty() -> Status {
    Status {
        text: "(EMPTY)".to_string(),
        style: Style {
            color: Some(super::util::Color::Yellow),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn default_hidden() -> Status {
    Status {
        text: "(HIDDEN)".to_string(),
        style: Style {
            color: Some(super::util::Color::Yellow),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn default_divergent() -> Status {
    Status {
        text: "(DIVERGENT)".to_string(),
        style: Style {
            color: Some(super::util::Color::Cyan),
            ..Default::default()
        },
        ..Default::default()
    }
}

#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[derive(Deserialize, Serialize, Debug, Default)]
struct Status {
    #[serde(default)]
    /// Do not render this warning
    disabled: bool,
    /// The text that should be printed when the working copy has the given state.
    text: String,
    #[serde(flatten, default)]
    style: Style,
}

impl Default for State {
    fn default() -> Self {
        Self {
            separator: default_separator(),
            conflict: default_conflict(),
            divergent: default_divergent(),
            hidden: default_hidden(),
            empty: default_empty(),
            immutable: default_immutable(),
        }
    }
}

impl State {
    pub fn print(
        &self,
        io: &mut impl Write,
        data: &crate::JJData,
        module_separator: &str,
    ) -> Result<(), CommandError> {
        let mut first = true;
        if !self.conflict.disabled && matches!(data.commit.warnings.conflict, Some(true)) {
            self.conflict.style.print(io, None)?;
            first = false;
            write!(io, "{}", self.conflict.text)?;
        }
        if !self.divergent.disabled && matches!(data.commit.warnings.divergent, Some(true)) {
            if !first {
                write!(io, "{}", self.separator)?;
            }
            first = false;
            self.divergent.style.print(io, None)?;
            write!(io, "{}", self.divergent.text)?;
        }
        if !self.hidden.disabled && matches!(data.commit.warnings.hidden, Some(true)) {
            if !first {
                write!(io, "{}", self.separator)?;
            }
            first = false;
            self.hidden.style.print(io, None)?;
            write!(io, "{}", self.hidden.text)?;
        }
        if !self.immutable.disabled && matches!(data.commit.warnings.immutable, Some(true)) {
            if !first {
                write!(io, "{}", self.separator)?;
            }
            first = false;
            self.immutable.style.print(io, None)?;
            write!(io, "{}", self.immutable.text)?;
        }
        if !self.empty.disabled && matches!(data.commit.warnings.empty, Some(true)) {
            if !first {
                write!(io, "{}", self.separator)?;
            }
            first = false;
            self.empty.style.print(io, None)?;
            write!(io, "{}", self.empty.text)?;
        }
        if !first {
            write!(io, "{module_separator}")?;
        }
        Ok(())
    }
    pub fn parse(
        &self,
        command_helper: &jj_cli::cli_util::CommandHelper,
        state: &mut crate::State,
        data: &mut crate::JJData,
        global: &super::GlobalConfig,
    ) -> Result<(), CommandError> {
        if !self.empty.disabled && data.commit.warnings.empty.is_none() {
            data.commit.warnings.empty = state.commit_is_empty(command_helper)?;
        }
        if !self.conflict.disabled && data.commit.warnings.conflict.is_none() {
            data.commit.warnings.conflict = state
                .commit(command_helper)?
                .as_ref()
                .map(|c| c.has_conflict());
        }

        self.parse_hidden_and_divergent(command_helper, state, data, global)?;

        if !self.immutable.disabled
            && data.commit.warnings.immutable.is_none()
            && let Some(commit_id) = state.commit_id(command_helper)?.clone()
        {
            let workspace_helper = state.workspace_helper(command_helper)?;
            let revs = workspace_helper
                .parse_revset(&Ui::null(), &RevisionArg::from("immutable()".to_string()))?;

            let immutable = revs.evaluate_to_commit_ids()?;

            let immutable_commit_id = commit_id.clone();
            data.commit.warnings.immutable = Some(pollster::block_on(immutable.any(move |id| {
                std::future::ready(id.as_ref().is_ok_and(|id| id == &immutable_commit_id))
            })));
        }

        Ok(())
    }
    pub fn parse_hidden_and_divergent(
        &self,
        command_helper: &jj_cli::cli_util::CommandHelper,
        state: &mut crate::State,
        data: &mut crate::JJData,
        _global: &super::GlobalConfig,
    ) -> Result<(), CommandError> {
        if (!self.hidden.disabled && data.commit.warnings.hidden.is_none())
            || (!self.divergent.disabled && data.commit.warnings.divergent.is_none())
        {
            let repo = state.repo(command_helper)?;
            let Some(commit) = state.commit(command_helper)? else {
                return Ok(());
            };
            let change_id = commit.change_id();
            let change = repo.resolve_change_id(change_id)?;

            let resolved_len = change.as_ref().map(|commits| {
                commits
                    .targets
                    .iter()
                    .filter(|(_, state)| *state == ResolvedChangeState::Visible)
                    .count()
            });
            let (hidden, divergent) = classify_change_resolution(resolved_len);

            if !self.hidden.disabled
                && data.commit.warnings.hidden.is_none()
                && let Some(v) = hidden
            {
                data.commit.warnings.hidden = Some(v);
            }
            if !self.divergent.disabled
                && data.commit.warnings.divergent.is_none()
                && let Some(v) = divergent
            {
                data.commit.warnings.divergent = Some(v);
            }
        }
        Ok(())
    }
}

// Pure helper to classify change-id resolution into hidden/divergent flags.
// None or 0 → hidden; 1 → none; >=2 → divergent.
pub(crate) fn classify_change_resolution(len: Option<usize>) -> (Option<bool>, Option<bool>) {
    match len {
        None => (Some(true), None),
        Some(0) => (Some(true), None),
        Some(1) => (None, None),
        Some(_) => (None, Some(true)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::strip_ansi;

    #[test]
    fn classify_none_is_hidden() {
        let (hidden, div) = classify_change_resolution(None);
        assert_eq!(hidden, Some(true));
        assert_eq!(div, None);
    }

    #[test]
    fn classify_zero_is_hidden() {
        let (hidden, div) = classify_change_resolution(Some(0));
        assert_eq!(hidden, Some(true));
        assert_eq!(div, None);
    }

    #[test]
    fn classify_one_is_neither() {
        let (hidden, div) = classify_change_resolution(Some(1));
        assert_eq!(hidden, None);
        assert_eq!(div, None);
    }

    #[test]
    fn classify_many_is_divergent() {
        let (hidden, div) = classify_change_resolution(Some(3));
        assert_eq!(hidden, None);
        assert_eq!(div, Some(true));
    }

    #[test]
    fn prints_in_order_with_separators_and_module_sep() {
        let s = State::default();
        let mut data = crate::JJData::default();
        data.commit.warnings.conflict = Some(true);
        data.commit.warnings.divergent = Some(true);
        data.commit.warnings.hidden = Some(true);
        data.commit.warnings.immutable = Some(true);
        data.commit.warnings.empty = Some(true);

        let mut out = Vec::new();
        s.print(&mut out, &data, "/").unwrap();
        let text = strip_ansi(&out);
        assert!(text.starts_with("(CONFLICT) (DIVERGENT) (HIDDEN) (IMMUTABLE) (EMPTY)/"));
    }

    #[test]
    fn state_prints_nothing_when_all_flags_empty() {
        let s = State::default();
        let data = crate::JJData::default();
        let mut out = Vec::new();
        s.print(&mut out, &data, "/").unwrap();
        assert_eq!(strip_ansi(&out), "");
    }

    #[test]
    fn state_prints_two_flags_with_separator_and_module_end() {
        let s = State::default();
        let mut data = crate::JJData::default();
        data.commit.warnings.divergent = Some(true);
        data.commit.warnings.hidden = Some(true);
        let mut out = Vec::new();
        s.print(&mut out, &data, "/").unwrap();
        assert_eq!(strip_ansi(&out), "(DIVERGENT) (HIDDEN)/");
    }

    #[test]
    fn disabled_hidden_is_not_rendered() {
        let mut s = State::default();
        // Access private field since test module is in the same file/module
        s.hidden.disabled = true;
        let mut data = crate::JJData::default();
        data.commit.warnings.hidden = Some(true);
        let mut out = Vec::new();
        s.print(&mut out, &data, "/").unwrap();
        assert_eq!(strip_ansi(&out), "");
    }
}
