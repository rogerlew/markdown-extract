use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

use globset::GlobMatcher;
use markdown_doc_config::{Config, LintIgnore, LintRule, SeverityLevel, TocSettings};
use markdown_doc_format::LintFinding;
use markdown_doc_parser::{DocumentSection, ParserContext};
use percent_encoding::percent_decode_str;
use pulldown_cmark::{Event, Options, Parser, Tag};
use rayon::prelude::*;
use strsim::normalized_levenshtein;

use crate::OperationError;

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
    pub schema_provider: &'a dyn SchemaProvider,
}

/// Trait describing schema requirements made available to the required-sections rule.
pub trait SchemaProvider: Send + Sync {
    fn required_sections(&self, _path: &Path) -> Option<SchemaMatch> {
        None
    }
}

/// Placeholder schema match representation for future integration.
#[derive(Clone, Debug, Default)]
pub struct SchemaMatch {
    pub required_headings: Vec<SchemaRequirement>,
}

/// Individual schema requirement entry.
#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub struct SchemaRequirement {
    pub heading: String,
    pub minimum_depth: Option<usize>,
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
        schema_provider: input.schema_provider,
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
    severity: SeverityLevel,
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
    schema_provider: &'a dyn SchemaProvider,
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
            let severity = config.lint.severity_for(*rule);
            if severity == SeverityLevel::Ignore {
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
                severity,
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
        let matchers = ignores
            .iter()
            .filter(|ignore| ignore.rules.contains(&active.rule))
            .map(|ignore| ignore.path.glob().compile_matcher())
            .collect();
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

        let findings = (active.executor)(&snapshot, env);
        for finding in findings {
            results.push(LintFinding {
                rule: active.rule,
                path: snapshot.relative_path.clone(),
                line: finding.line,
                message: finding.message,
                severity: active.severity,
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
        parse_toc_block(&self.contents, settings)
    }
}

#[derive(Clone)]
struct TocBlock {
    start_line: usize,
    entries: Vec<TocEntry>,
}

#[derive(Clone)]
struct TocEntry {
    anchor: String,
    text: String,
    line: usize,
}

fn compute_line_offsets(contents: &str) -> Vec<usize> {
    let mut offsets = Vec::new();
    offsets.push(0);
    for (idx, ch) in contents.char_indices() {
        if ch == '\n' {
            offsets.push(idx + 1);
        }
    }
    offsets
}

fn byte_to_line(byte: usize, offsets: &[usize]) -> usize {
    match offsets.binary_search(&byte) {
        Ok(idx) => idx + 1,
        Err(idx) => idx,
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
    if let Some(schema) = env
        .schema_provider
        .required_sections(&snapshot.relative_path)
    {
        if !schema.required_headings.is_empty() {
            // Placeholder stub until Agent 6 delivers schema matcher.
        }
    }
    Vec::new()
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

fn parse_toc_block(contents: &str, settings: &TocSettings) -> Option<TocBlock> {
    let mut in_block = false;
    let mut start_line = 0usize;
    let mut entries = Vec::new();

    for (idx, line) in contents.lines().enumerate() {
        let trimmed = line.trim();
        if !in_block {
            if trimmed == settings.start_marker {
                in_block = true;
                start_line = idx + 1;
            }
            continue;
        }

        if trimmed == settings.end_marker {
            return Some(TocBlock {
                start_line,
                entries,
            });
        }

        if let Some(entry) = parse_toc_entry(line, idx + 1) {
            entries.push(entry);
        }
    }

    None
}

fn parse_toc_entry(line: &str, line_number: usize) -> Option<TocEntry> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with(['-', '*', '+']) {
        return None;
    }
    let after_bullet = trimmed[1..].trim_start();
    if !after_bullet.starts_with('[') {
        return None;
    }
    let end_text = after_bullet.find(']')?;
    let text = after_bullet[1..end_text].trim().to_string();
    let remaining = after_bullet[end_text + 1..].trim_start();
    if !remaining.starts_with('(') {
        return None;
    }
    let end_paren = remaining.find(')')?;
    let target = remaining[1..end_paren].trim();
    if !target.starts_with('#') {
        return None;
    }

    Some(TocEntry {
        anchor: normalize_anchor_fragment(&target[1..]),
        text,
        line: line_number,
    })
}

fn is_external(target: &str) -> bool {
    let lower = target.to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
        || lower.starts_with("tel:")
        || lower.starts_with("data:")
}

fn is_markdown_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".md") || lower.ends_with(".markdown")
}

fn split_link_target(target: &str) -> (&str, Option<&str>) {
    if let Some((path, anchor)) = target.split_once('#') {
        if path.is_empty() {
            ("", Some(anchor))
        } else {
            (path, Some(anchor))
        }
    } else if let Some(stripped) = target.strip_prefix('#') {
        ("", Some(stripped))
    } else {
        (target, None)
    }
}

fn normalize_anchor_fragment(fragment: &str) -> String {
    percent_decode_str(fragment)
        .decode_utf8_lossy()
        .trim()
        .to_ascii_lowercase()
}

struct ResolvedPath {
    relative: PathBuf,
    absolute: PathBuf,
}

fn resolve_relative_path(base: &Path, target: &str, root: &Path) -> ResolvedPath {
    let base_absolute = root.join(base);
    let base_dir = base_absolute.parent().unwrap_or(root);
    let mut combined = if target.starts_with('/') {
        root.join(target.trim_start_matches('/'))
    } else {
        base_dir.join(target)
    };
    combined = normalize_path(combined);
    let relative = combined
        .strip_prefix(root)
        .map(|path| path.to_path_buf())
        .unwrap_or_else(|_| combined.clone());
    ResolvedPath {
        relative,
        absolute: combined,
    }
}

fn normalize_path(path: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                normalized.pop();
            }
            Component::CurDir => {}
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}
