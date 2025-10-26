//! Output renderers for markdown-doc commands.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Local, Utc};
use std::collections::HashSet;

use markdown_doc_config::{Config, LintRule, SeverityLevel};
use serde::Serialize;

/// Supported catalog output formats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CatalogFormat {
    Markdown,
    Json,
}

/// Supported lint output formats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LintFormat {
    Plain,
    Json,
    Sarif,
}

/// Lightweight heading summary for catalog rendering.
#[derive(Clone, Debug)]
pub struct HeadingSummary {
    pub level: usize,
    pub text: String,
    pub anchor: String,
}

/// Catalog entry describing headings for a single document.
#[derive(Clone, Debug)]
pub struct CatalogEntry {
    pub path: PathBuf,
    pub headings: Vec<HeadingSummary>,
}

/// Aggregate catalog render data.
#[derive(Clone, Debug)]
pub struct CatalogRenderData {
    pub generated_at: SystemTime,
    pub entries: Vec<CatalogEntry>,
}

/// Individual lint finding ready for rendering.
#[derive(Clone, Debug)]
pub struct LintFinding {
    pub rule: LintRule,
    pub path: PathBuf,
    pub line: usize,
    pub message: String,
    pub severity: SeverityLevel,
}

/// Aggregated lint render data.
#[derive(Clone, Debug)]
pub struct LintRenderData {
    pub files_scanned: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub findings: Vec<LintFinding>,
}

/// Public renderer that transforms operation outputs into user-facing strings.
pub struct Renderer {
    #[allow(dead_code)]
    config: Config,
}

impl Renderer {
    /// Build a renderer from configuration.
    pub fn from_config(config: Config) -> Self {
        Self { config }
    }

    /// Render catalog in Markdown format.
    pub fn render_catalog_markdown(&self, data: &CatalogRenderData) -> String {
        let timestamp = DateTime::<Utc>::from(data.generated_at)
            .with_timezone(&Local)
            .to_rfc3339();

        let mut output = String::new();
        output.push_str("# Documentation Catalog\n\n");
        output.push_str(&format!("Last updated: {}\n\n", timestamp));
        output.push_str("## Catalog\n\n");

        for entry in &data.entries {
            let path_str = normalize_path_display(&entry.path);
            output.push_str(&format!("- [{}]({})\n", path_str, path_str));
        }

        if !data.entries.is_empty() {
            output.push_str("\n---\n\n");
        }

        for (idx, entry) in data.entries.iter().enumerate() {
            let path_str = normalize_path_display(&entry.path);
            output.push_str(&format!("## {}\n\n", path_str));

            for heading in &entry.headings {
                let indent = "  ".repeat(heading.level.saturating_sub(1));
                output.push_str(&format!(
                    "{}- [{}](#{})\n",
                    indent, heading.text, heading.anchor
                ));
            }

            if idx + 1 != data.entries.len() {
                output.push_str("\n---\n\n");
            }
        }

        if !data.entries.is_empty() {
            output.push('\n');
        }

        output
    }

    /// Render catalog in JSON format.
    pub fn render_catalog_json(&self, data: &CatalogRenderData) -> serde_json::Result<String> {
        #[derive(Serialize)]
        struct JsonHeading<'a> {
            level: usize,
            text: &'a str,
            anchor: &'a str,
        }

        #[derive(Serialize)]
        struct JsonFile<'a> {
            path: String,
            headings: Vec<JsonHeading<'a>>,
        }

        #[derive(Serialize)]
        struct CatalogJson<'a> {
            last_updated: String,
            file_count: usize,
            files: Vec<JsonFile<'a>>,
        }

        let timestamp = DateTime::<Utc>::from(data.generated_at)
            .with_timezone(&Local)
            .to_rfc3339();

        let files = data
            .entries
            .iter()
            .map(|entry| JsonFile {
                path: normalize_path_display(&entry.path).into_owned(),
                headings: entry
                    .headings
                    .iter()
                    .map(|heading| JsonHeading {
                        level: heading.level,
                        text: heading.text.as_str(),
                        anchor: heading.anchor.as_str(),
                    })
                    .collect(),
            })
            .collect();

        let json = CatalogJson {
            last_updated: timestamp,
            file_count: data.entries.len(),
            files,
        };

        serde_json::to_string_pretty(&json)
    }

    /// Render lint results in plain-text format.
    pub fn render_lint_plain(&self, report: &LintRenderData) -> String {
        let mut output = String::new();

        for finding in &report.findings {
            let marker = match finding.severity {
                SeverityLevel::Error => "❌",
                SeverityLevel::Warning => "⚠️ ",
                SeverityLevel::Ignore => "ℹ️ ",
            };

            output.push_str(&format!(
                "{} {}:{} [{}] {}\n",
                marker,
                normalize_path_display(&finding.path),
                finding.line,
                finding.rule.as_str(),
                finding.message
            ));
        }

        if !report.findings.is_empty() {
            output.push('\n');
        }

        if report.error_count == 0 && report.warning_count == 0 {
            output.push_str(&format!(
                "✅ {} files validated, 0 errors, 0 warnings\n",
                report.files_scanned
            ));
        } else {
            output.push_str(&format!(
                "✅ {} files validated, {} errors, {} warnings\n",
                report.files_scanned, report.error_count, report.warning_count
            ));
        }

        output
    }

    /// Render lint results as JSON.
    pub fn render_lint_json(&self, report: &LintRenderData) -> serde_json::Result<String> {
        #[derive(Serialize)]
        struct Summary {
            files_scanned: usize,
            errors: usize,
            warnings: usize,
        }

        #[derive(Serialize)]
        struct JsonFinding<'a> {
            rule: &'a str,
            severity: &'a str,
            file: String,
            line: usize,
            message: &'a str,
        }

        #[derive(Serialize)]
        struct LintJson<'a> {
            summary: Summary,
            findings: Vec<JsonFinding<'a>>,
        }

        let findings = report
            .findings
            .iter()
            .map(|finding| JsonFinding {
                rule: finding.rule.as_str(),
                severity: severity_label(finding.severity),
                file: normalize_path_display(&finding.path).into_owned(),
                line: finding.line,
                message: finding.message.as_str(),
            })
            .collect();

        let json = LintJson {
            summary: Summary {
                files_scanned: report.files_scanned,
                errors: report.error_count,
                warnings: report.warning_count,
            },
            findings,
        };

        serde_json::to_string_pretty(&json)
    }

    /// Render lint results as SARIF v2.1.0.
    pub fn render_lint_sarif(&self, report: &LintRenderData) -> serde_json::Result<String> {
        #[derive(Serialize)]
        struct Message<'a> {
            text: &'a str,
        }

        #[derive(Serialize)]
        struct Region {
            start_line: usize,
        }

        #[derive(Serialize)]
        struct ArtifactLocation<'a> {
            uri: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            uri_base_id: Option<&'a str>,
        }

        #[derive(Serialize)]
        struct PhysicalLocation<'a> {
            artifact_location: ArtifactLocation<'a>,
            region: Region,
        }

        #[derive(Serialize)]
        struct Location<'a> {
            physical_location: PhysicalLocation<'a>,
        }

        #[derive(Serialize)]
        struct ResultEntry<'a> {
            rule_id: &'a str,
            level: &'a str,
            message: Message<'a>,
            locations: Vec<Location<'a>>,
        }

        #[derive(Serialize)]
        struct Rule<'a> {
            id: &'a str,
            name: &'a str,
            full_description: Message<'a>,
        }

        #[derive(Serialize)]
        struct Driver<'a> {
            name: &'a str,
            rules: Vec<Rule<'a>>,
        }

        #[derive(Serialize)]
        struct Tool<'a> {
            driver: Driver<'a>,
        }

        #[derive(Serialize)]
        struct Run<'a> {
            tool: Tool<'a>,
            results: Vec<ResultEntry<'a>>,
        }

        #[derive(Serialize)]
        struct Sarif<'a> {
            version: &'a str,
            #[serde(rename = "$schema")]
            schema: &'a str,
            runs: Vec<Run<'a>>,
        }

        let mut rule_set: HashSet<LintRule> = HashSet::new();

        let results = report
            .findings
            .iter()
            .map(|finding| {
                rule_set.insert(finding.rule);
                ResultEntry {
                    rule_id: finding.rule.as_str(),
                    level: sarif_level(finding.severity),
                    message: Message {
                        text: finding.message.as_str(),
                    },
                    locations: vec![Location {
                        physical_location: PhysicalLocation {
                            artifact_location: ArtifactLocation {
                                uri: normalize_path_display(&finding.path).into_owned(),
                                uri_base_id: Some("PROJECT_ROOT"),
                            },
                            region: Region {
                                start_line: finding.line,
                            },
                        },
                    }],
                }
            })
            .collect();

        let mut rules: Vec<Rule<'_>> = rule_set
            .into_iter()
            .map(|rule| Rule {
                id: rule.as_str(),
                name: rule_display_name(rule),
                full_description: Message {
                    text: rule_description(rule),
                },
            })
            .collect();

        rules.sort_by_key(|rule| rule.id);

        let sarif = Sarif {
            version: "2.1.0",
            schema: "https://json.schemastore.org/sarif-2.1.0.json",
            runs: vec![Run {
                tool: Tool {
                    driver: Driver {
                        name: "markdown-doc",
                        rules,
                    },
                },
                results,
            }],
        };

        serde_json::to_string_pretty(&sarif)
    }
}

fn normalize_path_display(path: &Path) -> std::borrow::Cow<'_, str> {
    use std::borrow::Cow;
    match path.to_str() {
        Some(s) => Cow::Borrowed(s),
        None => Cow::Owned(path.display().to_string()),
    }
}

fn severity_label(severity: SeverityLevel) -> &'static str {
    match severity {
        SeverityLevel::Error => "error",
        SeverityLevel::Warning => "warning",
        SeverityLevel::Ignore => "info",
    }
}

fn sarif_level(severity: SeverityLevel) -> &'static str {
    match severity {
        SeverityLevel::Error => "error",
        SeverityLevel::Warning => "warning",
        SeverityLevel::Ignore => "note",
    }
}

fn rule_display_name(rule: LintRule) -> &'static str {
    match rule {
        LintRule::BrokenLinks => "Broken Links",
        LintRule::BrokenAnchors => "Broken Anchors",
        LintRule::DuplicateAnchors => "Duplicate Anchors",
        LintRule::HeadingHierarchy => "Heading Hierarchy",
        LintRule::RequiredSections => "Required Sections",
        LintRule::TocSync => "TOC Sync",
    }
}

fn rule_description(rule: LintRule) -> &'static str {
    match rule {
        LintRule::BrokenLinks => "Internal markdown links must reference existing files.",
        LintRule::BrokenAnchors => "Inline anchors must resolve to existing headings.",
        LintRule::DuplicateAnchors => "Heading anchor slugs must be unique within a document.",
        LintRule::HeadingHierarchy => {
            "Heading levels must not skip levels or exceed configured depth."
        }
        LintRule::RequiredSections => "Documents must include schema-defined required sections.",
        LintRule::TocSync => {
            "Declared tables of contents must reflect the current heading structure."
        }
    }
}
