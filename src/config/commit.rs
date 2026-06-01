use std::io::Write;

use jj_cli::command_error::CommandError;
use jj_lib::id_prefix::IdPrefixIndex;
#[cfg(feature = "json-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::util::{Color, Style};

/// Prints the working copies commit text
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[derive(Deserialize, Serialize, Debug)]
pub struct Commit {
    /// A prefix that will be printed when the current commit is empty and the previous commit is shown.
    #[serde(default = "default_previous_message_symbol")]
    previous_message_symbol: char,
    /// Maximum length the commit text will be truncated to.
    #[serde(default = "default_max_length")]
    max_length: Option<usize>,
    /// Show the previous commit's description when the current commit is empty.
    #[serde(default)]
    show_previous_if_empty: bool,
    /// The text that should be printed when the current revision has no description yet
    #[serde(default = "default_empty_text")]
    empty_text: String,
    /// Controls how the commit text is rendered.
    #[serde(flatten)]
    style: Style,
    /// Do not render quotes around the description
    #[serde(default = "default_surround_with_quotes")]
    surround_with_quotes: bool,
    /// Controls if and how the change id should be shown.
    change: Option<Style>,
    /// Controls if and how the commit id should be shown.
    commit: Option<Style>,
    /// Controls how the non-unique part of ids should be shown.
    #[serde(default = "default_non_unique_style")]
    non_unique: Style,
}

fn default_previous_message_symbol() -> char {
    '⇣'
}

fn default_non_unique_style() -> Style {
    Style {
        color: Some(Color::Black),
        ..Default::default()
    }
}

fn default_unique_change_style() -> Style {
    Style {
        color: Some(Color::Magenta),
        ..Default::default()
    }
}

fn default_unique_commit_style() -> Style {
    Style {
        color: Some(Color::Blue),
        ..Default::default()
    }
}

fn default_max_length() -> Option<usize> {
    Some(24)
}
fn default_empty_text() -> String {
    "n/a".to_string()
}

fn default_surround_with_quotes() -> bool {
    true
}

impl Default for Commit {
    fn default() -> Self {
        Self {
            previous_message_symbol: default_previous_message_symbol(),
            show_previous_if_empty: false,
            style: Default::default(),
            max_length: default_max_length(),
            empty_text: default_empty_text(),
            surround_with_quotes: true,
            change: None,
            commit: None,
            non_unique: default_non_unique_style(),
        }
    }
}

impl Commit {
    pub fn print(
        &self,
        io: &mut impl Write,
        data: &crate::JJData,
        module_separator: &str,
    ) -> Result<(), CommandError> {
        let mut first = true;
        if let (Some(change), Some((change_id, change_idx))) =
            (&self.change, &data.commit.change_id)
        {
            change.print(io, default_unique_change_style())?;
            print_id(io, &change_id.to_string(), *change_idx, &self.non_unique)?;
            first = false;
        }
        if let (Some(commit), Some((commit_id, commit_idx))) =
            (&self.commit, &data.commit.commit_id)
        {
            if !first {
                write!(io, " ")?;
            }
            commit.print(io, default_unique_commit_style())?;
            print_id(io, &commit_id.to_string(), *commit_idx, &self.non_unique)?;
            first = false;
        }

        let Some(desc) = data.commit.desc.as_ref() else {
            return Ok(());
        };

        if !first {
            write!(io, " ")?;
        }

        let first_line = desc
            .split_once(['\r', '\n'])
            .map(|(line, _rest)| line)
            .unwrap_or(desc);

        self.style.print(io, None)?;

        if !desc.is_empty() {
            crate::print_ansi_truncated(
                self.max_length,
                io,
                first_line,
                self.surround_with_quotes,
            )?;
        } else {
            crate::print_ansi_truncated(
                self.max_length,
                io,
                &self.empty_text,
                self.surround_with_quotes,
            )?;
        }
        if data.commit.ahead {
            write!(io, "{}", self.previous_message_symbol)?;
        }
        write!(io, "{module_separator}")?;
        Ok(())
    }
    pub(crate) fn parse(
        &self,
        command_helper: &jj_cli::cli_util::CommandHelper,
        state: &mut crate::State,
        data: &mut crate::JJData,
        global: &super::GlobalConfig,
    ) -> Result<(), CommandError> {
        self.resolve_desc(command_helper, state, data, global)?;
        if self.commit.is_some() {
            self.resolve_commit_id(command_helper, state, data)?;
        }
        if self.change.is_some() {
            self.resolve_change_id(command_helper, state, data)?;
        }

        Ok(())
    }

    fn resolve_desc(
        &self,
        command_helper: &jj_cli::cli_util::CommandHelper,
        state: &mut crate::State,
        data: &mut crate::JJData,
        _global: &super::GlobalConfig,
    ) -> Result<(), CommandError> {
        if data.commit.desc.is_some() {
            return Ok(());
        }
        let Some(commit) = state.commit(command_helper)? else {
            return Ok(());
        };
        let description = commit.description().to_string();
        if description.is_empty() && self.show_previous_if_empty {
            let parents = state.parent_commits(command_helper)?;
            if parents.len() == 1
                && let Some(parent) = parents.first()
            {
                data.commit.desc = Some(parent.description().to_string());
                data.commit.ahead = true;
            }
        } else {
            data.commit.desc = Some(description);
        }
        Ok(())
    }

    fn resolve_commit_id(
        &self,
        command_helper: &jj_cli::cli_util::CommandHelper,
        state: &mut crate::State,
        data: &mut crate::JJData,
    ) -> Result<(), CommandError> {
        if data.commit.commit_id.is_some() {
            return Ok(());
        }
        let repo = state.repo(command_helper)?;
        let Some(commit) = state.commit(command_helper)? else {
            return Ok(());
        };
        let commit_id = commit.id().clone();
        let commit_idx =
            IdPrefixIndex::empty().shortest_commit_prefix_len(repo.as_ref(), &commit_id)?;
        data.commit.commit_id = Some((commit_id, commit_idx));
        Ok(())
    }

    fn resolve_change_id(
        &self,
        command_helper: &jj_cli::cli_util::CommandHelper,
        state: &mut crate::State,
        data: &mut crate::JJData,
    ) -> Result<(), CommandError> {
        if data.commit.change_id.is_some() {
            return Ok(());
        }
        let repo = state.repo(command_helper)?;
        let Some(commit) = state.commit(command_helper)? else {
            return Ok(());
        };
        let change_id = commit.change_id().clone();
        let change_idx =
            IdPrefixIndex::empty().shortest_change_prefix_len(repo.as_ref(), &change_id)?;
        data.commit.change_id = Some((change_id, change_idx));
        Ok(())
    }

    // Private helper used by production and tests; ensures idempotent behavior.
    #[cfg(test)]
    fn set_desc_if_missing(&self, data: &mut crate::JJData, desc: Option<String>) {
        if data.commit.desc.is_none()
            && let Some(d) = desc
        {
            data.commit.desc = Some(d);
        }
    }

    #[cfg(test)]
    fn parse_impl<F>(&self, data: &mut crate::JJData, get_desc: F) -> Result<(), CommandError>
    where
        F: FnOnce() -> Option<String>,
    {
        if data.commit.desc.is_some() {
            return Ok(());
        }
        let desc = get_desc();
        self.set_desc_if_missing(data, desc);
        Ok(())
    }
}

fn print_id(
    io: &mut impl Write,
    id: &str,
    unique_len: usize,
    non_unique_style: &Style,
) -> Result<(), CommandError> {
    let short_id = &id[..8.min(id.len())];
    let split_at = unique_len.min(short_id.len());
    let (unique, non_unique) = short_id.split_at(split_at);
    write!(io, "{unique}")?;
    if !non_unique.is_empty() {
        non_unique_style.print(io, default_non_unique_style())?;
        write!(io, "{non_unique}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::strip_ansi;

    #[test]
    fn prints_first_line_only() {
        let mut data = crate::JJData::default();
        data.commit.desc = Some("first line\nsecond".to_string());
        let c = Commit::default();
        let mut out = Vec::new();
        c.print(&mut out, &data, "/").unwrap();
        assert_eq!(strip_ansi(&out), "\"first line\"/");
    }

    #[test]
    fn prints_empty_fallback_when_no_description() {
        let mut data = crate::JJData::default();
        data.commit.desc = Some("".to_string());
        let c = Commit::default();
        let mut out = Vec::new();
        c.print(&mut out, &data, "/").unwrap();
        assert_eq!(strip_ansi(&out), "\"n/a\"/");
    }

    #[test]
    fn truncates_and_quotes_respected() {
        let mut data = crate::JJData::default();
        data.commit.desc = Some("abcdef".to_string());
        let c = Commit {
            max_length: Some(3),
            surround_with_quotes: false,
            ..Default::default()
        };
        let mut out = Vec::new();
        c.print(&mut out, &data, "/").unwrap();
        assert_eq!(strip_ansi(&out), "ab…/");
    }

    #[test]
    fn commit_prints_nothing_when_desc_is_none() {
        let mut data = crate::JJData::default();
        data.commit.desc = None;
        let c = Commit::default();
        let mut out = Vec::new();
        c.print(&mut out, &data, "/").unwrap();
        assert_eq!(strip_ansi(&out), "");
    }

    #[test]
    fn carriage_return_split_is_respected() {
        let mut data = crate::JJData::default();
        data.commit.desc = Some("first\rsecond".to_string());
        let c = Commit::default();
        let mut out = Vec::new();
        c.print(&mut out, &data, "/").unwrap();
        assert_eq!(strip_ansi(&out), "\"first\"/");
    }

    #[test]
    fn set_desc_if_missing_sets_desc_only_when_none() {
        let c = Commit::default();
        let mut data = crate::JJData::default();
        // Case 1: desc is None, incoming desc is Some
        c.set_desc_if_missing(&mut data, Some("msg".to_string()));
        assert_eq!(data.commit.desc.as_deref(), Some("msg"));

        // Case 2: desc already set; new desc should not override
        c.set_desc_if_missing(&mut data, Some("new".to_string()));
        assert_eq!(data.commit.desc.as_deref(), Some("msg"));

        // Case 3: desc None, incoming desc None -> remains None
        let mut data2 = crate::JJData::default();
        c.set_desc_if_missing(&mut data2, None);
        assert!(data2.commit.desc.is_none());
    }

    #[test]
    fn commit_parse_impl_sets_and_short_circuits() {
        let c = Commit::default();
        let mut data = crate::JJData::default();
        let mut called = false;
        c.parse_impl(&mut data, || {
            called = true;
            Some("msg".to_string())
        })
        .unwrap();
        assert!(called);
        assert_eq!(data.commit.desc.as_deref(), Some("msg"));

        // Second call should not invoke the closure when desc is already set
        c.parse_impl(&mut data, || panic!("should not be called"))
            .unwrap();
    }

    #[test]
    fn print_id_styles_non_unique_suffix() {
        let mut out = Vec::new();
        print_id(&mut out, "abcdef123456", 3, &default_non_unique_style()).unwrap();
        assert_eq!(strip_ansi(&out), "abcdef12");
    }

    #[test]
    fn print_appends_previous_message_symbol_when_ahead() {
        let mut data = crate::JJData::default();
        data.commit.desc = Some("parent description".to_string());
        data.commit.ahead = true;
        let c = Commit::default();
        let mut out = Vec::new();
        c.print(&mut out, &data, "/").unwrap();
        assert_eq!(strip_ansi(&out), "\"parent description\"⇣/");
    }
}
