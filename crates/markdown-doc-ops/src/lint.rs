use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use globset::GlobMatcher;
use markdown_doc_config::{
    Config, LintIgnore, LintIgnoreRules, LintRule, SeverityLevel, TocSettings,
};
use markdown_doc_format::LintFinding;
use markdown_doc_parser::{DocumentSection, ParserContext};
use pulldown_cmark::{Event, Options, Parser, Tag};
use rayon::prelude::*;
use strsim::normalized_levenshtein;

use crate::{
    anchors::normalize_anchor_fragment,
    lines::{byte_to_line, compute_line_offsets},
    paths::{is_external, is_markdown_path, resolve_relative_path, split_link_target},
    schema::SchemaEngine,
    toc,
    toc::{TocBlock, TocEntry},
    OperationError,
};

/// Result produced by the lint engine prior to rendering.
pub(crate) struct LintResult {
    pub findings: Vec<LintFinding>,
    pub files_scanned: usize,
    pub error_count: usize,
    pub warning_count: usize,
}

/// Input bundle for executing lint rules.
pub(crate) struct LintRunInput<'a> {
    pub config: &'a Config,
    pub parser: &'a ParserContext,
    pub targets: &'a [PathBuf],
    pub root: &'a Path,
    pub schema_engine: &'a SchemaEngine,
}

/// Execute lint rules across the provided targets.
pub(crate) fn run(input: LintRunInput<'_>) -> Result<LintResult, OperationError> {
    let active_rules = build_active_rules(input.config);
    if active_rules.is_empty() {
        return Ok(LintResult {
            findings: Vec::new(),
            files_scanned: input.targets.len(),
            error_count: 0,
            warning_count: 0,
        });
    }

    let ignore_map = build_ignore_map(&input.config.lint.ignore, &active_rules);
    let environment = LintEnvironment {
        config: input.config,
        parser: input.parser,
        root: input.root,
        anchor_cache: Arc::new(AnchorCache::default()),
        schema_engine: input.schema_engine,
    };

    let findings = input
        .targets
        .par_iter()
        .map(|path| process_file(path, &active_rules, &ignore_map, &environment))
        .try_reduce(Vec::new, |mut acc, mut next| {
            acc.append(&mut next);
            Ok(acc)
        })?;

    let mut sorted = findings;
    sorted.sort_by(|a, b| match a.path.cmp(&b.path) {
        std::cmp::Ordering::Equal => match a.line.cmp(&b.line) {
            std::cmp::Ordering::Equal => a.rule.as_str().cmp(b.rule.as_str()),
            other => other,
        },
        other => other,
    });

    let (errors, warnings) = sorted.iter().fold((0usize, 0usize), |mut acc, finding| {
        match finding.severity {
            SeverityLevel::Error => acc.0 += 1,
            SeverityLevel::Warning => acc.1 += 1,
            SeverityLevel::Ignore => {}
        }
        acc
    });

    Ok(LintResult {
        findings: sorted,
        files_scanned: input.targets.len(),
        error_count: errors,
        warning_count: warnings,
    })
}

#[derive(Clone)]
struct ActiveRule {
    rule: LintRule,
    executor: RuleExecutor,
}

type RuleExecutor = fn(&FileSnapshot, &LintEnvironment) -> Vec<RuleFinding>;

#[derive(Clone)]
struct RuleFinding {
    line: usize,
    message: String,
}

struct LintEnvironment<'a> {
    config: &'a Config,
    parser: &'a ParserContext,
    root: &'a Path,
    anchor_cache: Arc<AnchorCache>,
    schema_engine: &'a SchemaEngine,
}

#[derive(Default)]
struct AnchorCache {
    inner: Mutex<HashMap<PathBuf, Vec<String>>>,
}

impl AnchorCache {
    fn anchors_for(
        &self,
        ctx: &LintEnvironment<'_>,
        relative_path: &Path,
    ) -> Result<Option<Vec<String>>, OperationError> {
        if let Some(cached) = self.inner.lock().unwrap().get(relative_path) {
            return Ok(Some(cached.clone()));
        }

        let sections = match ctx.parser.sections_for_path(relative_path) {
            Ok(sections) => sections,
            Err(err) => {
                let absolute = ctx.root.join(relative_path);
                if err.kind() == std::io::ErrorKind::NotFound {
                    return Ok(None);
                }
                return Err(OperationError::Io {
                    path: absolute,
                    source: err,
                });
            }
        };

        let anchors: Vec<String> = sections
            .into_iter()
            .map(|section| section.heading.anchor)
            .collect();

        let mut guard = self.inner.lock().unwrap();
        guard.insert(relative_path.to_path_buf(), anchors.clone());
        Ok(Some(anchors))
    }
}

fn build_active_rules(config: &Config) -> Vec<ActiveRule> {
    config
        .lint
        .rules
        .iter()
        .filter_map(|rule| {
            if !config.lint.is_rule_enabled(*rule) {
                return None;
            }

            let executor = match rule {
                LintRule::BrokenLinks => evaluate_broken_links as RuleExecutor,
                LintRule::BrokenAnchors => evaluate_broken_anchors as RuleExecutor,
                LintRule::DuplicateAnchors => evaluate_duplicate_anchors as RuleExecutor,
                LintRule::HeadingHierarchy => evaluate_heading_hierarchy as RuleExecutor,
                LintRule::RequiredSections => evaluate_required_sections as RuleExecutor,
                LintRule::TocSync => evaluate_toc_sync as RuleExecutor,
            };

            Some(ActiveRule {
                rule: *rule,
                executor,
            })
        })
        .collect()
}

fn build_ignore_map(
    ignores: &[LintIgnore],
    active_rules: &[ActiveRule],
) -> HashMap<LintRule, Vec<GlobMatcher>> {
    let mut map = HashMap::new();
    for active in active_rules {
        let mut matchers = Vec::new();
        for ignore in ignores {
            match &ignore.rules {
                LintIgnoreRules::All => matchers.push(ignore.matcher.clone()),
                LintIgnoreRules::Specific(rules) => {
                    if rules.contains(&active.rule) {
                        matchers.push(ignore.matcher.clone());
                    }
                }
            }
        }
        map.insert(active.rule, matchers);
    }
    map
}

fn process_file(
    relative_path: &Path,
    rules: &[ActiveRule],
    ignore_map: &HashMap<LintRule, Vec<GlobMatcher>>,
    env: &LintEnvironment,
) -> Result<Vec<LintFinding>, OperationError> {
    let absolute = env.root.join(relative_path);
    let contents = match std::fs::read_to_string(&absolute) {
        Ok(contents) => contents,
        Err(err) => {
            if err.kind() == std::io::ErrorKind::NotFound {
                return Ok(vec![LintFinding {
                    rule: rules
                        .first()
                        .map(|rule| rule.rule)
                        .unwrap_or(LintRule::BrokenLinks),
                    path: relative_path.to_path_buf(),
                    line: 0,
                    message: format!("failed to read file: {err}"),
                    severity: SeverityLevel::Error,
                }]);
            }
            return Err(OperationError::Io {
                path: absolute,
                source: err,
            });
        }
    };

    let snapshot = FileSnapshot::from_contents(relative_path, contents, env.parser)?;

    let mut results = Vec::new();
    for active in rules {
        let ignored = ignore_map
            .get(&active.rule)
            .map(|matchers| matches_ignored(matchers, &snapshot.relative_path))
            .unwrap_or(false);
        if ignored {
            continue;
        }

        let severity = env
            .config
            .lint
            .severity_for_path(&snapshot.relative_path, active.rule);

        if severity == SeverityLevel::Ignore {
            continue;
        }

        let findings = (active.executor)(&snapshot, env);
        for finding in findings {
            results.push(LintFinding {
                rule: active.rule,
                path: snapshot.relative_path.clone(),
                line: finding.line,
                message: finding.message,
                severity,
            });
        }
    }

    Ok(results)
}

fn matches_ignored(matchers: &[GlobMatcher], path: &Path) -> bool {
    matchers.iter().any(|matcher| matcher.is_match(path))
}

#[derive(Clone)]
struct FileSnapshot {
    relative_path: PathBuf,
    contents: Arc<String>,
    sections: Vec<DocumentSection>,
    line_offsets: Arc<Vec<usize>>,
    anchors: Vec<AnchorInfo>,
    links: Vec<LinkReference>,
}

#[derive(Clone)]
struct AnchorInfo {
    anchor: String,
    normalized: String,
    depth: usize,
    line: usize,
}

#[derive(Clone)]
struct LinkReference {
    target: String,
    line: usize,
}

impl FileSnapshot {
    fn from_contents(
        relative: &Path,
        contents: String,
        parser: &ParserContext,
    ) -> Result<Self, OperationError> {
        let line_offsets = Arc::new(compute_line_offsets(&contents));
        let sections = parser.sections_from_str(relative, &contents);

        let anchors = sections
            .iter()
            .map(|section| AnchorInfo {
                anchor: section.heading.anchor.clone(),
                normalized: section.heading.normalized.clone(),
                depth: section.heading.depth,
                line: byte_to_line(section.heading.byte_range.start, &line_offsets),
            })
            .collect();

        let links = extract_links(&contents, &line_offsets);

        Ok(FileSnapshot {
            relative_path: relative.to_path_buf(),
            contents: Arc::new(contents),
            sections,
            line_offsets,
            anchors,
            links,
        })
    }

    fn normalized_anchor_set(&self) -> HashSet<String> {
        self.anchors
            .iter()
            .map(|anchor| anchor.anchor.clone())
            .collect()
    }

    fn toc_block(&self, settings: &TocSettings) -> Option<TocBlock> {
        toc::locate_block(&self.contents, settings)
    }
}

fn extract_links(contents: &str, offsets: &[usize]) -> Vec<LinkReference> {
    let mut links = Vec::new();
    let parser = Parser::new_ext(contents, Options::all());
    for (event, range) in parser.into_offset_iter() {
        if let Event::Start(Tag::Link(_, dest, _)) = event {
            let target = dest.to_string().trim().to_string();
            let line = byte_to_line(range.start, offsets);
            links.push(LinkReference { target, line });
        }
    }
    links
}

fn evaluate_broken_links(snapshot: &FileSnapshot, env: &LintEnvironment) -> Vec<RuleFinding> {
    let mut findings = Vec::new();
    for link in &snapshot.links {
        if link.target.is_empty() {
            continue;
        }
        if is_external(&link.target) {
            continue;
        }

        let (path_part, _) = split_link_target(&link.target);
        if path_part.is_empty() {
            continue;
        }
        if !is_markdown_path(path_part) {
            continue;
        }

        let resolved = resolve_relative_path(&snapshot.relative_path, path_part, env.root);
        if !resolved.absolute.exists() {
            findings.push(RuleFinding {
                line: link.line,
                message: format!("Broken link to '{}'", link.target),
            });
        }
    }
    findings
}

fn evaluate_broken_anchors(snapshot: &FileSnapshot, env: &LintEnvironment) -> Vec<RuleFinding> {
    let mut findings = Vec::new();
    let local_anchors = snapshot.normalized_anchor_set();

    for link in &snapshot.links {
        if link.target.is_empty() {
            continue;
        }

        let (path_part, anchor_part) = split_link_target(&link.target);
        if anchor_part.is_none() {
            continue;
        }

        let raw_anchor = anchor_part.unwrap();
        if raw_anchor.is_empty() {
            continue;
        }

        let normalized_anchor = normalize_anchor_fragment(raw_anchor);

        if path_part.is_empty() {
            if !local_anchors.contains(&normalized_anchor) {
                findings.push(anchor_missing_finding(
                    link.line,
                    &normalized_anchor,
                    &link.target,
                    &snapshot.anchors,
                ));
            }
            continue;
        }

        if !is_markdown_path(path_part) {
            continue;
        }

        let resolved = resolve_relative_path(&snapshot.relative_path, path_part, env.root);
        if !resolved.absolute.exists() {
            // Broken links rule will flag missing file.
            continue;
        }

        let anchors = match env.anchor_cache.anchors_for(env, &resolved.relative) {
            Ok(Some(list)) => list,
            Ok(None) => continue,
            Err(_) => continue,
        };

        let anchor_set: HashSet<String> = anchors.iter().cloned().collect();
        if !anchor_set.contains(&normalized_anchor) {
            findings.push(RuleFinding {
                line: link.line,
                message: format!(
                    "Missing anchor '#{}' in link to '{}'",
                    normalized_anchor, link.target
                ),
            });
        }
    }

    findings
}

fn anchor_missing_finding(
    line: usize,
    anchor: &str,
    target: &str,
    anchors: &[AnchorInfo],
) -> RuleFinding {
    let suggestion = anchors
        .iter()
        .map(|candidate| {
            (
                candidate.anchor.as_str(),
                normalized_levenshtein(anchor, candidate.anchor.as_str()),
            )
        })
        .filter(|(_, score)| *score > 0.6)
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(slug, _)| slug.to_string());

    let mut message = format!("Missing anchor '#{}' referenced by '{}'", anchor, target);
    if let Some(suggestion) = suggestion {
        message.push_str(&format!(". Did you mean '#{}'?", suggestion));
    }

    RuleFinding { line, message }
}

fn evaluate_duplicate_anchors(snapshot: &FileSnapshot, _env: &LintEnvironment) -> Vec<RuleFinding> {
    let mut findings = Vec::new();
    let mut map: HashMap<&str, Vec<&AnchorInfo>> = HashMap::new();
    for anchor in &snapshot.anchors {
        map.entry(anchor.anchor.as_str()).or_default().push(anchor);
    }

    for (slug, entries) in map {
        if entries.len() > 1 {
            let first = entries[0];
            for duplicate in entries.iter().skip(1) {
                findings.push(RuleFinding {
                    line: duplicate.line,
                    message: format!(
                        "Duplicate anchor '{}' (first defined at line {})",
                        slug, first.line
                    ),
                });
            }
        }
    }
    findings
}

fn evaluate_heading_hierarchy(snapshot: &FileSnapshot, env: &LintEnvironment) -> Vec<RuleFinding> {
    let mut findings = Vec::new();
    let mut previous_depth: Option<usize> = None;
    let max_depth = env.config.lint.max_heading_depth as usize;

    for section in &snapshot.sections {
        let depth = section.heading.depth;
        let line = byte_to_line(section.heading.byte_range.start, &snapshot.line_offsets);

        if depth > max_depth {
            findings.push(RuleFinding {
                line,
                message: format!(
                    "Heading '{}' exceeds max depth {}",
                    section.heading.raw, max_depth
                ),
            });
        }

        if let Some(prev) = previous_depth {
            if depth > prev + 1 {
                findings.push(RuleFinding {
                    line,
                    message: format!(
                        "Heading '{}' skips from level {} to {}",
                        section.heading.raw, prev, depth
                    ),
                });
            }
        }

        previous_depth = Some(depth);
    }

    findings
}

fn evaluate_required_sections(snapshot: &FileSnapshot, env: &LintEnvironment) -> Vec<RuleFinding> {
    let schema = env.schema_engine.schema_for_path(&snapshot.relative_path);
    let check = env
        .schema_engine
        .check(schema, &snapshot.sections, &snapshot.line_offsets);

    check
        .violations
        .into_iter()
        .map(|violation| RuleFinding {
            line: violation.line,
            message: violation.message,
        })
        .collect()
}

fn evaluate_toc_sync(snapshot: &FileSnapshot, env: &LintEnvironment) -> Vec<RuleFinding> {
    let mut findings = Vec::new();
    let Some(block) = snapshot.toc_block(&env.config.lint.toc) else {
        return findings;
    };

    let document_heads: Vec<&AnchorInfo> = snapshot
        .anchors
        .iter()
        .filter(|anchor| anchor.depth >= 2)
        .collect();

    let toc_anchors: Vec<&TocEntry> = block.entries.iter().collect();

    let toc_anchor_set: HashSet<&str> = toc_anchors
        .iter()
        .map(|entry| entry.anchor.as_str())
        .collect();
    let document_anchor_set: HashSet<&str> = document_heads
        .iter()
        .map(|anchor| anchor.anchor.as_str())
        .collect();

    for heading in &document_heads {
        if !toc_anchor_set.contains(heading.anchor.as_str()) {
            findings.push(RuleFinding {
                line: block.start_line,
                message: format!(
                    "TOC missing entry for heading '{}' (#{}).",
                    heading.normalized, heading.anchor
                ),
            });
        }
    }

    for entry in &toc_anchors {
        if !document_anchor_set.contains(entry.anchor.as_str()) {
            findings.push(RuleFinding {
                line: entry.line,
                message: format!(
                    "TOC entry '{}' references unknown heading '#{}'.",
                    entry.text, entry.anchor
                ),
            });
        }
    }

    let min_len = document_heads.len().min(toc_anchors.len());
    for idx in 0..min_len {
        let expected = document_heads[idx].anchor.as_str();
        let actual = toc_anchors[idx].anchor.as_str();
        if expected != actual {
            findings.push(RuleFinding {
                line: toc_anchors[idx].line,
                message: format!(
                    "TOC entry '{}' is out of order (expected '#{}').",
                    toc_anchors[idx].text, expected
                ),
            });
            break;
        }
    }

    findings
}

// Path helpers now live in crate::paths.
