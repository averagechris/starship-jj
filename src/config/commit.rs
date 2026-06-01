use std::io::Write;

use jj_cli::command_error::CommandError;
#[cfg(feature = "json-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::util::Style;

/// Prints the working copies commit text
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[derive(Deserialize, Serialize, Debug)]
pub struct Commit {
    /// Maximum length the commit text will be truncated to.
    #[serde(default = "default_max_length")]
    max_length: Option<usize>,
    /// The text that should be printed when the current revision has no description yet
    #[serde(default = "default_empty_text")]
    empty_text: String,
    /// Controls how the commit text is rendered.
    #[serde(flatten)]
    style: Style,
    /// Do not render quotes around the description
    #[serde(default = "default_surround_with_quotes")]
    surround_with_quotes: bool,
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
            style: Default::default(),
            max_length: default_max_length(),
            empty_text: default_empty_text(),
            surround_with_quotes: true,
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
        let Some(desc) = data.commit.desc.as_ref() else {
            return Ok(());
        };

        let first_line = desc
            .split_once(['\r', '\n'])
            .map(|(line, _rest)| line)
            .unwrap_or(desc);

        if !first_line.is_empty() {
            self.style.print(io, None)?;

            crate::print_ansi_truncated(
                self.max_length,
                io,
                first_line,
                self.surround_with_quotes,
            )?;
            write!(io, "{module_separator}")?;
        } else {
            self.style.print(io, None)?;
            crate::print_ansi_truncated(
                self.max_length,
                io,
                &self.empty_text,
                self.surround_with_quotes,
            )?;
            write!(io, "{module_separator}")?;
        }
        Ok(())
    }
    pub(crate) fn parse(
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
        let desc = Some(commit.description().to_string());
        self.set_desc_if_missing(data, desc);
        Ok(())
    }

    // Private helper used by production and tests; ensures idempotent behavior.
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
}
