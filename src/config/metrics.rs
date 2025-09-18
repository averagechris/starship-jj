use std::io::Write;

use jj_cli::command_error::CommandError;
#[cfg(feature = "json-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::util::{Color, Style};

/// Prints the amount of changes in the working copy
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[derive(Deserialize, Serialize, Debug)]
pub struct Metrics {
    /// Controls how the changes are rendered, use {added}, {removed} and {changed} to render the number of changes.
    #[serde(default = "default_template")]
    template: String,

    // added_files: Style,
    // removed_files: Style,
    /// Controlls how the number of changed files is rendered.
    #[serde(default = "default_changed_files")]
    changed_files: Metric,

    /// Controlls how the number of added lines is rendered.
    #[serde(default = "default_added_lines")]
    added_lines: Metric,
    /// Controlls how the number of removed lines is rendered.
    #[serde(default = "default_removed_lines")]
    removed_lines: Metric,

    #[serde(flatten, default = "default_style")]
    style: Style,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            style: default_style(),
            template: default_template(),
            changed_files: default_changed_files(),
            added_lines: default_added_lines(),
            removed_lines: default_removed_lines(),
        }
    }
}

fn default_removed_lines() -> Metric {
    Metric {
        style: default_removed_style(),
        prefix: "-".to_string(),
        ..Default::default()
    }
}

fn default_removed_style() -> Style {
    Style {
        color: Some(Color::Red),
        ..Default::default()
    }
}

fn default_added_lines() -> Metric {
    Metric {
        style: default_added_style(),
        prefix: "+".to_string(),
        ..Default::default()
    }
}

fn default_added_style() -> Style {
    Style {
        color: Some(Color::Green),
        ..Default::default()
    }
}

fn default_changed_files() -> Metric {
    Metric {
        style: default_changed_style(),
        ..Default::default()
    }
}

fn default_changed_style() -> Style {
    Style {
        color: Some(Color::Cyan),
        ..Default::default()
    }
}

fn default_template() -> String {
    "[{changed} {added}{removed}]".to_string()
}

fn default_style() -> Style {
    Style {
        color: Some(Color::Magenta),
        ..Default::default()
    }
}

#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[derive(Deserialize, Serialize, Debug, Default)]
struct Metric {
    #[serde(default)]
    prefix: String,
    #[serde(default)]
    suffix: String,
    #[serde(flatten)]
    style: Style,
}
impl Metric {
    fn format(
        &self,
        number: usize,
        global_style: &Style,
        fallback: impl Into<Option<Style>>,
    ) -> String {
        format!(
            "{}{}{}{}{}",
            self.style.format(fallback),
            self.prefix,
            number,
            self.suffix,
            global_style.format(default_style()),
        )
    }
}

#[derive(Debug, Serialize)]
struct Context {
    added: String,
    removed: String,
    changed: String,
}

impl Metrics {
    pub fn print(
        &self,
        io: &mut impl Write,
        data: &crate::JJData,
        module_separator: &str,
    ) -> Result<(), CommandError> {
        let Some(diff) = &data.commit.diff else {
            return Ok(());
        };

        let context = Context {
            added: self
                .added_lines
                .format(diff.lines_added, &self.style, default_added_style()),
            removed: self.removed_lines.format(
                diff.lines_removed,
                &self.style,
                default_removed_style(),
            ),
            changed: self.changed_files.format(
                diff.files_changed,
                &self.style,
                default_changed_style(),
            ),
        };
        let mut tiny_template = tinytemplate::TinyTemplate::new();
        tiny_template
            .add_template("template", &self.template)
            .map_err(|e| {
                CommandError::with_message(
                    jj_cli::command_error::CommandErrorKind::Internal,
                    "template",
                    e,
                )
            })?;
        let s = tiny_template.render("template", &context).map_err(|e| {
            CommandError::with_message(
                jj_cli::command_error::CommandErrorKind::Internal,
                "template",
                e,
            )
        })?;

        self.style.print(io, default_style())?;

        write!(io, "{s}{module_separator}")?;

        Ok(())
    }
    pub(crate) fn parse(
        &self,
        command_helper: &jj_cli::cli_util::CommandHelper,
        state: &mut crate::State,
        data: &mut crate::JJData,
        _global: &super::GlobalConfig,
    ) -> Result<(), CommandError> {
        if data.commit.diff.is_some() {
            return Ok(());
        }

        if state.commit_is_empty(command_helper)? == Some(true) {
            data.commit.diff = Some(Default::default());
            return Ok(());
        }

        let Some(stats) = state.diff_stats(command_helper)? else {
            return Ok(());
        };

        let diff = crate::CommitDiff {
            files_changed: stats.entries().len(),
            lines_added: stats.count_total_added(),
            lines_removed: stats.count_total_removed(),
        };

        data.commit.diff = Some(diff);

        Ok(())
    }
}

#[cfg(test)]
mod print_tests {
    use super::*;
    use crate::testutil::strip_ansi;

    #[test]
    fn prints_numbers_into_template() {
        let m = Metrics::default();
        let mut data = crate::JJData::default();
        data.commit.diff = Some(crate::CommitDiff {
            files_changed: 3,
            lines_added: 10,
            lines_removed: 2,
        });
        let mut out = Vec::new();
        m.print(&mut out, &data, "/").unwrap();
        let text = strip_ansi(&out);
        assert!(text.contains("["));
        assert!(text.contains("3"));
        assert!(text.contains("+10"));
        assert!(text.contains("-2"));
        assert!(text.ends_with("/"));
    }

    #[test]
    fn invalid_template_returns_error() {
        let m = Metrics {
            template: "{{{{".to_string(),
            ..Default::default()
        };
        let mut data = crate::JJData::default();
        data.commit.diff = Some(crate::CommitDiff::default());
        let mut out = Vec::new();
        let err = m.print(&mut out, &data, "/").unwrap_err();
        let _ = err; // only assert it errors without depending on exact message
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SimpleStats {
    files_changed: usize,
    lines_added: usize,
    lines_removed: usize,
}

#[cfg(test)]
impl Metrics {
    fn parse_impl<FIsEmpty, FStats>(
        &self,
        data: &mut crate::JJData,
        is_empty: FIsEmpty,
        get_stats: FStats,
    ) -> Result<(), CommandError>
    where
        FIsEmpty: FnOnce() -> Result<Option<bool>, CommandError>,
        FStats: FnOnce() -> Result<Option<SimpleStats>, CommandError>,
    {
        if data.commit.diff.is_some() {
            return Ok(());
        }

        if is_empty()? == Some(true) {
            data.commit.diff = Some(Default::default());
            return Ok(());
        }

        let Some(stats) = get_stats()? else {
            return Ok(());
        };

        let diff = crate::CommitDiff {
            files_changed: stats.files_changed,
            lines_added: stats.lines_added,
            lines_removed: stats.lines_removed,
        };
        data.commit.diff = Some(diff);

        Ok(())
    }
}

#[cfg(test)]
mod parse_tests {
    use super::*;

    #[test]
    fn short_circuits_on_empty_commit() -> Result<(), CommandError> {
        let m = Metrics::default();
        let mut data = crate::JJData::default();

        m.parse_impl(
            &mut data,
            || Ok(Some(true)),
            || panic!("should not be called"),
        )?;

        let diff = data.commit.diff.as_ref().expect("diff should be set");
        assert_eq!(diff.files_changed, 0);
        assert_eq!(diff.lines_added, 0);
        assert_eq!(diff.lines_removed, 0);
        Ok(())
    }

    #[test]
    fn computes_stats_when_not_empty() -> Result<(), CommandError> {
        let m = Metrics::default();
        let mut data = crate::JJData::default();

        m.parse_impl(
            &mut data,
            || Ok(Some(false)),
            || {
                Ok(Some(SimpleStats {
                    files_changed: 3,
                    lines_added: 10,
                    lines_removed: 2,
                }))
            },
        )?;

        let diff = data.commit.diff.as_ref().expect("diff should be set");
        assert_eq!(diff.files_changed, 3);
        assert_eq!(diff.lines_added, 10);
        assert_eq!(diff.lines_removed, 2);
        Ok(())
    }
}
