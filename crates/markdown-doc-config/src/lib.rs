//! Configuration primitives and loader for the markdown-doc toolkit.
//!
//! The loader resolves configuration using the precedence stack described in
//! `markdown-doc.spec.md`:
//! override flag → working directory → git root → built-in defaults.
//! Parsed settings are normalised into typed structures so downstream crates
//! can operate without touching raw TOML.

use std::collections::{HashMap, HashSet};
use std::env;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobMatcher};
use serde::Deserialize;
use thiserror::Error;

const CONFIG_FILE_NAME: &str = ".markdown-doc.toml";

/// Complete configuration resolved from defaults and on-disk overrides.
#[derive(Clone, Debug)]
pub struct Config {
    pub project: ProjectSettings,
    pub catalog: CatalogSettings,
    pub lint: LintSettings,
    pub schemas: SchemaSettings,
    pub sources: ConfigSources,
}

/// Project-level settings that declare repository scope boundaries.
#[derive(Clone, Debug)]
pub struct ProjectSettings {
    pub name: Option<String>,
    pub root: PathBuf,
    pub exclude: PatternList,
}

/// Settings that govern the catalog command.
#[derive(Clone, Debug)]
pub struct CatalogSettings {
    pub output: PathBuf,
    pub include: PatternList,
    pub exclude: PatternList,
}

/// Settings covering lint behaviour and rule configuration.
#[derive(Clone, Debug)]
pub struct LintSettings {
    pub rules: Vec<LintRule>,
    pub severity: HashMap<LintRule, SeverityLevel>,
    pub severity_wildcard: Option<SeverityLevel>,
    pub severity_overrides: Vec<LintSeverityOverride>,
    pub max_heading_depth: u8,
    pub ignore: Vec<LintIgnore>,
    pub toc: TocSettings,
}

impl LintSettings {
    /// Returns the effective severity for `rule`, defaulting to `Error`.
    pub fn severity_for(&self, rule: LintRule) -> SeverityLevel {
        self.severity
            .get(&rule)
            .copied()
            .or(self.severity_wildcard)
            .unwrap_or(SeverityLevel::Error)
    }

    /// Returns the effective severity for `rule` when evaluating `path`.
    pub fn severity_for_path(&self, path: &Path, rule: LintRule) -> SeverityLevel {
        for override_entry in self.severity_overrides.iter().rev() {
            if override_entry.matcher.is_match(path) {
                if let Some(level) = override_entry.rules.get(&rule).copied() {
                    return level;
                }
                if let Some(level) = override_entry.wildcard {
                    return level;
                }
            }
        }
        self.severity_for(rule)
    }

    /// Determine whether the rule is enabled in any scope.
    pub fn is_rule_enabled(&self, rule: LintRule) -> bool {
        if self.severity_for(rule) != SeverityLevel::Ignore {
            return true;
        }

        for override_entry in &self.severity_overrides {
            if let Some(level) = override_entry.rules.get(&rule) {
                if *level != SeverityLevel::Ignore {
                    return true;
                }
            } else if let Some(level) = override_entry.wildcard {
                if level != SeverityLevel::Ignore {
                    return true;
                }
            }
        }

        false
    }
}

/// Path-scoped severity override rules.
#[derive(Clone, Debug)]
pub struct LintSeverityOverride {
    pub path: Pattern,
    pub matcher: GlobMatcher,
    pub rules: HashMap<LintRule, SeverityLevel>,
    pub wildcard: Option<SeverityLevel>,
    pub source: ConfigSource,
}

/// Configuration governing TOC marker detection.
#[derive(Clone, Debug)]
pub struct TocSettings {
    pub start_marker: String,
    pub end_marker: String,
}

/// Resolved schema configuration providing template definitions and pattern precedence.
#[derive(Clone, Debug)]
pub struct SchemaSettings {
    pub default_schema: String,
    pub schemas: HashMap<String, SchemaDefinition>,
    pub patterns: Vec<SchemaPattern>,
}

/// Individual schema template definition.
#[derive(Clone, Debug)]
pub struct SchemaDefinition {
    pub name: String,
    pub source: ConfigSource,
    pub patterns: Vec<Pattern>,
    pub required_sections: Vec<String>,
    pub allow_additional: bool,
    pub allow_empty: bool,
    pub min_sections: Option<u32>,
    pub min_heading_level: Option<u8>,
    pub max_heading_level: Option<u8>,
    pub require_top_level_heading: Option<bool>,
}

/// Pattern-to-schema association with pre-computed specificity ordering.
#[derive(Clone, Debug)]
pub struct SchemaPattern {
    pub schema: String,
    pub matcher: Pattern,
    specificity: PatternSpecificity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct PatternSpecificity {
    segments: usize,
    literal_chars: usize,
}

/// Pattern plus compiled matcher helper.
#[derive(Clone, Debug)]
pub struct Pattern {
    original: String,
    glob: Glob,
}

impl Pattern {
    fn new(source: ConfigSource, value: String) -> Result<Self, ConfigValidationError> {
        match Glob::new(&value) {
            Ok(glob) => Ok(Pattern {
                original: value,
                glob,
            }),
            Err(err) => Err(ConfigValidationError::new(
                Some(source),
                format!("invalid glob pattern '{value}': {err}"),
            )),
        }
    }

    pub fn original(&self) -> &str {
        &self.original
    }

    pub fn glob(&self) -> &Glob {
        &self.glob
    }
}

/// Ordered list of glob patterns.
#[derive(Clone, Debug, Default)]
pub struct PatternList {
    patterns: Vec<Pattern>,
}

impl PatternList {
    fn new(patterns: Vec<Pattern>) -> Self {
        PatternList { patterns }
    }

    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Pattern> {
        self.patterns.iter()
    }
}

/// Supported lint rules. Keep the list in sync with the specification.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LintRule {
    BrokenLinks,
    BrokenAnchors,
    DuplicateAnchors,
    HeadingHierarchy,
    RequiredSections,
    TocSync,
}

impl LintRule {
    pub const ALL: &'static [LintRule] = &[
        LintRule::BrokenLinks,
        LintRule::BrokenAnchors,
        LintRule::DuplicateAnchors,
        LintRule::HeadingHierarchy,
        LintRule::RequiredSections,
        LintRule::TocSync,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            LintRule::BrokenLinks => "broken-links",
            LintRule::BrokenAnchors => "broken-anchors",
            LintRule::DuplicateAnchors => "duplicate-anchors",
            LintRule::HeadingHierarchy => "heading-hierarchy",
            LintRule::RequiredSections => "required-sections",
            LintRule::TocSync => "toc-sync",
        }
    }
}

impl fmt::Display for LintRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for LintRule {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "broken-links" => Ok(LintRule::BrokenLinks),
            "broken-anchors" => Ok(LintRule::BrokenAnchors),
            "duplicate-anchors" => Ok(LintRule::DuplicateAnchors),
            "heading-hierarchy" => Ok(LintRule::HeadingHierarchy),
            "required-sections" => Ok(LintRule::RequiredSections),
            "toc-sync" => Ok(LintRule::TocSync),
            _ => Err(()),
        }
    }
}

/// Severity configuration surfaced to lint consumers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SeverityLevel {
    Error,
    Warning,
    Ignore,
}

impl fmt::Display for SeverityLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            SeverityLevel::Error => "error",
            SeverityLevel::Warning => "warning",
            SeverityLevel::Ignore => "ignore",
        };
        f.write_str(label)
    }
}

impl std::str::FromStr for SeverityLevel {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "error" => Ok(SeverityLevel::Error),
            "warning" => Ok(SeverityLevel::Warning),
            "ignore" => Ok(SeverityLevel::Ignore),
            _ => Err(()),
        }
    }
}

/// Ignore rule describing which lint rules to suppress for a glob path.
#[derive(Clone, Debug)]
pub struct LintIgnore {
    pub path: Pattern,
    pub matcher: GlobMatcher,
    pub rules: LintIgnoreRules,
    pub source: ConfigSource,
}

/// Target set for lint ignore entries.
#[derive(Clone, Debug)]
pub enum LintIgnoreRules {
    All,
    Specific(Vec<LintRule>),
}

/// Provenance information for resolved configuration.
#[derive(Clone, Debug)]
pub struct ConfigSources {
    pub working_directory: PathBuf,
    pub layers: Vec<ConfigSource>,
}

/// Specific layer of configuration (default/git/local/override).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigSource {
    pub kind: ConfigSourceKind,
    pub path: Option<PathBuf>,
    pub base_dir: PathBuf,
}

impl ConfigSource {
    fn default(base_dir: PathBuf) -> Self {
        ConfigSource {
            kind: ConfigSourceKind::Default,
            path: None,
            base_dir,
        }
    }

    fn for_file(kind: ConfigSourceKind, path: PathBuf) -> Self {
        let base_dir = path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        ConfigSource {
            kind,
            path: Some(path),
            base_dir,
        }
    }

    fn describe(&self) -> String {
        match (&self.kind, &self.path) {
            (ConfigSourceKind::Default, _) => "built-in defaults".to_owned(),
            (kind, Some(path)) => format!("{} at {}", kind, path.display()),
            (kind, None) => kind.to_string(),
        }
    }
}

/// Kinds of configuration sources, ordered from lowest to highest precedence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigSourceKind {
    Default,
    GitRoot,
    Local,
    Override,
}

impl fmt::Display for ConfigSourceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            ConfigSourceKind::Default => "defaults",
            ConfigSourceKind::GitRoot => "git-root config",
            ConfigSourceKind::Local => "local config",
            ConfigSourceKind::Override => "override config",
        };
        f.write_str(label)
    }
}

/// Loader options, typically supplied by the CLI layer.
#[derive(Clone, Debug, Default)]
pub struct LoadOptions {
    pub override_path: Option<PathBuf>,
    pub working_dir: Option<PathBuf>,
}

impl LoadOptions {
    pub fn with_override_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.override_path = Some(path.into());
        self
    }

    pub fn with_working_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(path.into());
        self
    }
}

/// Errors surfaced while loading configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to resolve working directory {attempted}: {source}")]
    WorkingDirectory {
        attempted: PathBuf,
        source: io::Error,
    },
    #[error("override config {path} not found")]
    OverrideNotFound { path: PathBuf },
    #[error("failed to read config {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("failed to parse config {path}: {source}")]
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("configuration validation failed:\n{0}")]
    Validation(ConfigValidationErrors),
}

impl Config {
    /// Loads configuration using the precedence rules and returns typed settings.
    pub fn load(options: LoadOptions) -> Result<Self, ConfigError> {
        let working_dir = resolve_working_dir(options.working_dir)?;
        let override_path = options
            .override_path
            .map(|path| make_absolute(&path, &working_dir));

        if let Some(path) = &override_path {
            if !path.exists() {
                return Err(ConfigError::OverrideNotFound { path: path.clone() });
            }
        }

        let default_source = ConfigSource::default(working_dir.clone());
        let mut merged = PartialConfig::empty();
        merged.merge(defaults_layer(default_source.clone()));

        let mut source_layers = vec![default_source];

        let git_root = find_git_root(&working_dir);
        let git_config_path = git_root.as_ref().map(|root| root.join(CONFIG_FILE_NAME));
        let local_config_path = working_dir.join(CONFIG_FILE_NAME);

        if let Some(path) = git_config_path.as_ref() {
            if path.exists() && Some(path) != override_path.as_ref() && path != &local_config_path {
                let source = ConfigSource::for_file(ConfigSourceKind::GitRoot, path.clone());
                merged.merge(load_layer(path, source.clone())?);
                source_layers.push(source);
            }
        }

        if local_config_path.exists() && Some(&local_config_path) != override_path.as_ref() {
            let source = ConfigSource::for_file(ConfigSourceKind::Local, local_config_path.clone());
            merged.merge(load_layer(&local_config_path, source.clone())?);
            source_layers.push(source);
        }

        if let Some(path) = override_path {
            let source = ConfigSource::for_file(ConfigSourceKind::Override, path.clone());
            merged.merge(load_layer(&path, source.clone())?);
            source_layers.push(source);
        }

        let config = merged.finalize().map_err(ConfigError::Validation)?;
        Ok(Config {
            project: config.project,
            catalog: config.catalog,
            lint: config.lint,
            schemas: config.schemas,
            sources: ConfigSources {
                working_directory: working_dir,
                layers: source_layers,
            },
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        Config::load(LoadOptions::default()).unwrap_or_else(|err| {
            panic!("failed to load markdown-doc defaults: {err}");
        })
    }
}

fn resolve_working_dir(override_dir: Option<PathBuf>) -> Result<PathBuf, ConfigError> {
    match override_dir {
        Some(path) => fs::canonicalize(&path).map_err(|source| ConfigError::WorkingDirectory {
            attempted: path,
            source,
        }),
        None => env::current_dir().map_err(|source| ConfigError::WorkingDirectory {
            attempted: PathBuf::from("."),
            source,
        }),
    }
}

fn make_absolute(path: &Path, base: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

fn load_layer(path: &Path, source: ConfigSource) -> Result<PartialConfig, ConfigError> {
    let contents = fs::read_to_string(path).map_err(|source| ConfigError::Io {
        path: path.into(),
        source,
    })?;
    parse_layer(&contents, source).map_err(|err| match err {
        LayerParseError::Parse { source } => ConfigError::Parse {
            path: path.into(),
            source,
        },
    })
}

fn parse_layer(contents: &str, source: ConfigSource) -> Result<PartialConfig, LayerParseError> {
    let raw: RawConfig =
        toml::from_str(contents).map_err(|source| LayerParseError::Parse { source })?;
    Ok(raw.into_partial(source))
}

fn defaults_layer(source: ConfigSource) -> PartialConfig {
    let project = ProjectPartial {
        root: Some(Located::new(PathBuf::from("."), source.clone())),
        exclude: Some(Located::new(Vec::new(), source.clone())),
        ..ProjectPartial::default()
    };

    let catalog = CatalogPartial {
        output: Some(Located::new(
            PathBuf::from("DOC_CATALOG.md"),
            source.clone(),
        )),
        include_patterns: Some(Located::new(vec!["**/*.md".into()], source.clone())),
        exclude_patterns: Some(Located::new(
            vec!["**/node_modules/**".into(), "**/vendor/**".into()],
            source.clone(),
        )),
    };

    let lint = LintPartial {
        rules: Some(Located::new(vec!["broken-links".into()], source.clone())),
        max_heading_depth: Some(Located::new(4, source.clone())),
        toc_start_marker: Some(Located::new("<!-- toc -->".into(), source.clone())),
        toc_end_marker: Some(Located::new("<!-- tocstop -->".into(), source.clone())),
        ..LintPartial::default()
    };

    let default_schema = SchemaDefinitionPartial {
        source: Some(source.clone()),
        required_sections: Some(Located::new(Vec::new(), source.clone())),
        allow_additional: Some(Located::new(true, source.clone())),
        allow_empty: Some(Located::new(false, source.clone())),
        min_heading_level: Some(Located::new(1, source.clone())),
        max_heading_level: Some(Located::new(4, source.clone())),
        require_top_level_heading: Some(Located::new(true, source.clone())),
        ..SchemaDefinitionPartial::default()
    };

    let mut schemas = SchemasPartial::default();
    schemas.entries.insert("default".into(), default_schema);

    PartialConfig {
        project: Some(project),
        catalog: Some(catalog),
        lint: Some(lint),
        schemas: Some(schemas),
    }
}

fn find_git_root(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(dir) = current {
        if dir.join(".git").exists() {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}

#[derive(Debug)]
enum LayerParseError {
    Parse { source: toml::de::Error },
}

#[derive(Clone, Debug, Default)]
struct PartialConfig {
    project: Option<ProjectPartial>,
    catalog: Option<CatalogPartial>,
    lint: Option<LintPartial>,
    schemas: Option<SchemasPartial>,
}

impl PartialConfig {
    fn empty() -> Self {
        PartialConfig {
            project: None,
            catalog: None,
            lint: None,
            schemas: None,
        }
    }

    fn merge(&mut self, mut other: PartialConfig) {
        if let Some(other_project) = other.project.take() {
            match &mut self.project {
                Some(project) => project.merge(other_project),
                None => self.project = Some(other_project),
            }
        }

        if let Some(other_catalog) = other.catalog.take() {
            match &mut self.catalog {
                Some(catalog) => catalog.merge(other_catalog),
                None => self.catalog = Some(other_catalog),
            }
        }

        if let Some(other_lint) = other.lint.take() {
            match &mut self.lint {
                Some(lint) => lint.merge(other_lint),
                None => self.lint = Some(other_lint),
            }
        }

        if let Some(other_schemas) = other.schemas.take() {
            match &mut self.schemas {
                Some(schemas) => schemas.merge(other_schemas),
                None => self.schemas = Some(other_schemas),
            }
        }
    }

    fn finalize(self) -> Result<ResolvedConfig, ConfigValidationErrors> {
        let mut errors = Vec::new();

        let project_partial = self.project.unwrap_or_default();
        let project_root_loc = project_partial.root.unwrap_or_else(|| {
            Located::new(
                PathBuf::from("."),
                ConfigSource::default(PathBuf::from(".")),
            )
        });

        let project_root = resolve_path(&project_root_loc);
        let exclude_patterns = compile_patterns(
            project_partial.exclude.unwrap_or_default(),
            "project.exclude",
            &mut errors,
        );

        let catalog_partial = self.catalog.unwrap_or_default();
        let catalog_output_loc = catalog_partial.output.unwrap_or_else(|| {
            Located::new(
                PathBuf::from("DOC_CATALOG.md"),
                ConfigSource::default(PathBuf::from(".")),
            )
        });

        let catalog_output = resolve_path(&catalog_output_loc);
        let catalog_include = compile_patterns(
            catalog_partial.include_patterns.unwrap_or_default(),
            "catalog.include_patterns",
            &mut errors,
        );
        let catalog_exclude = compile_patterns(
            catalog_partial.exclude_patterns.unwrap_or_default(),
            "catalog.exclude_patterns",
            &mut errors,
        );

        let lint_partial = self.lint.unwrap_or_default();
        let rules_loc = lint_partial.rules.unwrap_or_else(|| {
            Located::new(
                vec!["broken-links".into()],
                ConfigSource::default(PathBuf::from(".")),
            )
        });
        let rules = parse_rules(rules_loc, &mut errors);

        let max_heading_depth = lint_partial
            .max_heading_depth
            .unwrap_or_else(|| Located::new(4, ConfigSource::default(PathBuf::from("."))));

        if max_heading_depth.value == 0 || max_heading_depth.value > 6 {
            errors.push(ConfigValidationError::new(
                Some(max_heading_depth.source.clone()),
                format!(
                    "lint.max_heading_depth must be between 1 and 6 (received {})",
                    max_heading_depth.value
                ),
            ));
        }

        let toc_start_marker = lint_partial.toc_start_marker.unwrap_or_else(|| {
            Located::new(
                "<!-- toc -->".to_string(),
                ConfigSource::default(PathBuf::from(".")),
            )
        });

        if toc_start_marker.value.trim().is_empty() {
            errors.push(ConfigValidationError::new(
                Some(toc_start_marker.source.clone()),
                "lint.toc_start_marker cannot be empty".into(),
            ));
        }

        let toc_end_marker = lint_partial.toc_end_marker.unwrap_or_else(|| {
            Located::new(
                "<!-- tocstop -->".to_string(),
                ConfigSource::default(PathBuf::from(".")),
            )
        });

        if toc_end_marker.value.trim().is_empty() {
            errors.push(ConfigValidationError::new(
                Some(toc_end_marker.source.clone()),
                "lint.toc_end_marker cannot be empty".into(),
            ));
        }

        let (severity, severity_wildcard) = parse_severity_map(lint_partial.severity, &mut errors);
        let severity_overrides =
            parse_severity_overrides(lint_partial.severity_overrides, &mut errors);
        let ignore = parse_ignore_list(lint_partial.ignore, &mut errors);

        let schemas_partial = self.schemas.unwrap_or_default();
        let schemas = finalize_schemas(schemas_partial, &mut errors);

        if !errors.is_empty() {
            return Err(ConfigValidationErrors(errors));
        }

        let toc_settings = TocSettings {
            start_marker: toc_start_marker.value,
            end_marker: toc_end_marker.value,
        };

        Ok(ResolvedConfig {
            project: ProjectSettings {
                name: project_partial.name.map(|name| name.value),
                root: project_root,
                exclude: PatternList::new(exclude_patterns),
            },
            catalog: CatalogSettings {
                output: catalog_output,
                include: PatternList::new(catalog_include),
                exclude: PatternList::new(catalog_exclude),
            },
            lint: LintSettings {
                rules,
                severity,
                severity_wildcard,
                severity_overrides,
                max_heading_depth: max_heading_depth.value,
                ignore,
                toc: toc_settings,
            },
            schemas,
        })
    }
}

#[derive(Clone, Debug, Default)]
struct ProjectPartial {
    name: Option<Located<String>>,
    root: Option<Located<PathBuf>>,
    exclude: Option<Located<Vec<String>>>,
}

impl ProjectPartial {
    fn merge(&mut self, other: ProjectPartial) {
        if other.name.is_some() {
            self.name = other.name;
        }
        if other.root.is_some() {
            self.root = other.root;
        }
        if other.exclude.is_some() {
            self.exclude = other.exclude;
        }
    }
}

#[derive(Clone, Debug, Default)]
struct CatalogPartial {
    output: Option<Located<PathBuf>>,
    include_patterns: Option<Located<Vec<String>>>,
    exclude_patterns: Option<Located<Vec<String>>>,
}

impl CatalogPartial {
    fn merge(&mut self, other: CatalogPartial) {
        if other.output.is_some() {
            self.output = other.output;
        }
        if other.include_patterns.is_some() {
            self.include_patterns = other.include_patterns;
        }
        if other.exclude_patterns.is_some() {
            self.exclude_patterns = other.exclude_patterns;
        }
    }
}

#[derive(Clone, Debug, Default)]
struct LintPartial {
    rules: Option<Located<Vec<String>>>,
    max_heading_depth: Option<Located<u8>>,
    severity: HashMap<String, Located<String>>,
    ignore: Vec<Located<LintIgnorePartial>>,
    severity_overrides: Vec<Located<LintSeverityOverridePartial>>,
    toc_start_marker: Option<Located<String>>,
    toc_end_marker: Option<Located<String>>,
}

impl LintPartial {
    fn merge(&mut self, other: LintPartial) {
        if other.rules.is_some() {
            self.rules = other.rules;
        }
        if other.max_heading_depth.is_some() {
            self.max_heading_depth = other.max_heading_depth;
        }
        if other.toc_start_marker.is_some() {
            self.toc_start_marker = other.toc_start_marker;
        }
        if other.toc_end_marker.is_some() {
            self.toc_end_marker = other.toc_end_marker;
        }
        for (key, value) in other.severity {
            self.severity.insert(key, value);
        }
        self.ignore.extend(other.ignore);
        self.severity_overrides.extend(other.severity_overrides);
    }
}

#[derive(Clone, Debug)]
struct LintIgnorePartial {
    path: String,
    rules: Vec<String>,
}

#[derive(Clone, Debug)]
struct LintSeverityOverridePartial {
    path: String,
    rules: HashMap<String, String>,
}

#[derive(Clone, Debug, Default)]
struct SchemasPartial {
    entries: HashMap<String, SchemaDefinitionPartial>,
}

impl SchemasPartial {
    fn merge(&mut self, other: SchemasPartial) {
        for (name, definition) in other.entries {
            self.entries
                .entry(name)
                .and_modify(|existing| existing.merge(definition.clone()))
                .or_insert(definition);
        }
    }
}

#[derive(Clone, Debug, Default)]
struct SchemaDefinitionPartial {
    source: Option<ConfigSource>,
    patterns: Option<Located<Vec<String>>>,
    required_sections: Option<Located<Vec<String>>>,
    allow_additional: Option<Located<bool>>,
    allow_empty: Option<Located<bool>>,
    min_sections: Option<Located<u32>>,
    min_heading_level: Option<Located<u8>>,
    max_heading_level: Option<Located<u8>>,
    require_top_level_heading: Option<Located<bool>>,
}

impl SchemaDefinitionPartial {
    fn merge(&mut self, other: SchemaDefinitionPartial) {
        if other.source.is_some() {
            self.source = other.source;
        }
        if other.patterns.is_some() {
            self.patterns = other.patterns;
        }
        if other.required_sections.is_some() {
            self.required_sections = other.required_sections;
        }
        if other.allow_additional.is_some() {
            self.allow_additional = other.allow_additional;
        }
        if other.allow_empty.is_some() {
            self.allow_empty = other.allow_empty;
        }
        if other.min_sections.is_some() {
            self.min_sections = other.min_sections;
        }
        if other.min_heading_level.is_some() {
            self.min_heading_level = other.min_heading_level;
        }
        if other.max_heading_level.is_some() {
            self.max_heading_level = other.max_heading_level;
        }
        if other.require_top_level_heading.is_some() {
            self.require_top_level_heading = other.require_top_level_heading;
        }
    }
}

#[derive(Clone, Debug)]
struct Located<T> {
    value: T,
    source: ConfigSource,
}

impl<T> Located<T> {
    fn new(value: T, source: ConfigSource) -> Self {
        Located { value, source }
    }
}

impl Default for Located<Vec<String>> {
    fn default() -> Self {
        Located::new(Vec::new(), ConfigSource::default(PathBuf::from(".")))
    }
}

fn resolve_path(located: &Located<PathBuf>) -> PathBuf {
    let path = &located.value;
    if path.is_absolute() {
        path.clone()
    } else {
        located.source.base_dir.join(path)
    }
}

fn compile_patterns(
    located: Located<Vec<String>>,
    context: &str,
    errors: &mut Vec<ConfigValidationError>,
) -> Vec<Pattern> {
    let mut patterns = Vec::new();
    for pattern in located.value {
        match Pattern::new(located.source.clone(), pattern.clone()) {
            Ok(compiled) => patterns.push(compiled),
            Err(err) => errors.push(err.with_context(context)),
        }
    }
    patterns
}

fn finalize_schemas(
    partial: SchemasPartial,
    errors: &mut Vec<ConfigValidationError>,
) -> SchemaSettings {
    if partial.entries.is_empty() {
        errors.push(
            ConfigValidationError::new(None, "at least one schema definition is required".into())
                .with_context("schemas"),
        );
    }

    let mut definitions: HashMap<String, SchemaDefinition> = HashMap::new();
    let mut pattern_entries: Vec<SchemaPattern> = Vec::new();
    let mut default_schema: Option<String> = None;

    for (name, definition) in partial.entries {
        let SchemaDefinitionPartial {
            source,
            patterns,
            required_sections,
            allow_additional,
            allow_empty,
            min_sections,
            min_heading_level,
            max_heading_level,
            require_top_level_heading,
        } = definition;

        let schema_source = source.unwrap_or_else(|| ConfigSource::default(PathBuf::from(".")));

        let compiled_patterns = patterns
            .map(|located| compile_patterns(located, &format!("schemas.{name}.patterns"), errors))
            .unwrap_or_default();

        let required_sections = required_sections
            .map(|located| located.value)
            .unwrap_or_default();

        let allow_additional = allow_additional
            .map(|located| located.value)
            .unwrap_or(true);
        let allow_empty = allow_empty.map(|located| located.value).unwrap_or(false);

        let min_sections_value = min_sections.map(|located| {
            let value = located.value;
            if value == 0 {
                errors.push(
                    ConfigValidationError::new(
                        Some(located.source.clone()),
                        format!("min_sections must be greater than 0 (received {value})"),
                    )
                    .with_context(format!("schemas.{name}.min_sections")),
                );
            }
            value
        });

        let min_heading_level_value = min_heading_level.map(|located| {
            let value = located.value;
            if value == 0 || value > 6 {
                errors.push(ConfigValidationError::new(
                    Some(located.source.clone()),
                    format!(
                        "schemas.{name}.min_heading_level must be between 1 and 6 (received {})",
                        value
                    ),
                ));
            }
            value
        });

        let max_heading_level_value = max_heading_level.map(|located| {
            let value = located.value;
            if value == 0 || value > 6 {
                errors.push(ConfigValidationError::new(
                    Some(located.source.clone()),
                    format!(
                        "schemas.{name}.max_heading_level must be between 1 and 6 (received {})",
                        value
                    ),
                ));
            }
            value
        });

        if let (Some(min), Some(max)) = (min_heading_level_value, max_heading_level_value) {
            if min > max {
                errors.push(
                    ConfigValidationError::new(
                        Some(schema_source.clone()),
                        format!(
                            "schemas.{name}: min_heading_level ({min}) must not exceed max_heading_level ({max})"
                        ),
                    ),
                );
            }
        }

        if name == "default" {
            default_schema = Some(name.clone());
        }

        let definition_entry = SchemaDefinition {
            name: name.clone(),
            source: schema_source.clone(),
            patterns: compiled_patterns.clone(),
            required_sections,
            allow_additional,
            allow_empty,
            min_sections: min_sections_value,
            min_heading_level: min_heading_level_value,
            max_heading_level: max_heading_level_value,
            require_top_level_heading: require_top_level_heading.map(|located| located.value),
        };

        for pattern in &compiled_patterns {
            pattern_entries.push(SchemaPattern {
                schema: name.clone(),
                matcher: pattern.clone(),
                specificity: pattern_specificity(pattern),
            });
        }

        definitions.insert(name, definition_entry);
    }

    if default_schema.is_none() {
        errors.push(
            ConfigValidationError::new(None, "schemas.default must be defined".into())
                .with_context("schemas"),
        );
        if definitions.is_empty() {
            let fallback_source = ConfigSource::default(PathBuf::from("."));
            definitions.insert(
                "default".into(),
                SchemaDefinition {
                    name: "default".into(),
                    source: fallback_source.clone(),
                    patterns: Vec::new(),
                    required_sections: Vec::new(),
                    allow_additional: true,
                    allow_empty: false,
                    min_sections: None,
                    min_heading_level: Some(1),
                    max_heading_level: Some(4),
                    require_top_level_heading: Some(true),
                },
            );
        }
    }

    pattern_entries.sort_by(|a, b| {
        b.specificity
            .cmp(&a.specificity)
            .then_with(|| a.schema.cmp(&b.schema))
            .then_with(|| a.matcher.original().cmp(b.matcher.original()))
    });

    SchemaSettings {
        default_schema: default_schema.unwrap_or_else(|| "default".into()),
        schemas: definitions,
        patterns: pattern_entries,
    }
}

fn pattern_specificity(pattern: &Pattern) -> PatternSpecificity {
    let text = pattern.original();
    let segments = text
        .split(&['/', '\\'][..])
        .filter(|segment| !segment.is_empty())
        .count();
    let literal_chars = text
        .chars()
        .filter(|ch| !matches!(ch, '*' | '?' | '[' | ']' | '{' | '}' | '!'))
        .count();
    PatternSpecificity {
        segments,
        literal_chars,
    }
}

fn parse_rules(
    located: Located<Vec<String>>,
    errors: &mut Vec<ConfigValidationError>,
) -> Vec<LintRule> {
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    for rule_name in located.value {
        match rule_name.parse::<LintRule>() {
            Ok(rule) => {
                if !seen.insert(rule) {
                    errors.push(
                        ConfigValidationError::new(
                            Some(located.source.clone()),
                            format!("duplicate lint rule '{rule}' in lint.rules"),
                        )
                        .with_context("lint.rules"),
                    );
                } else {
                    result.push(rule);
                }
            }
            Err(_) => errors.push(
                ConfigValidationError::new(
                    Some(located.source.clone()),
                    format!("unknown lint rule '{rule_name}'"),
                )
                .with_context("lint.rules"),
            ),
        }
    }
    result
}

fn parse_severity_map(
    raw: HashMap<String, Located<String>>,
    errors: &mut Vec<ConfigValidationError>,
) -> (HashMap<LintRule, SeverityLevel>, Option<SeverityLevel>) {
    let mut result = HashMap::new();
    let mut wildcard: Option<SeverityLevel> = None;
    for (rule_name, located_value) in raw {
        if rule_name == "*" {
            match located_value.value.parse::<SeverityLevel>() {
                Ok(level) => {
                    if wildcard.is_some() {
                        errors.push(
                            ConfigValidationError::new(
                                Some(located_value.source.clone()),
                                "duplicate wildcard entry in lint.severity".into(),
                            )
                            .with_context("lint.severity"),
                        );
                    } else {
                        wildcard = Some(level);
                    }
                }
                Err(_) => errors.push(
                    ConfigValidationError::new(
                        Some(located_value.source.clone()),
                        format!(
                            "invalid severity '{}' for wildcard entry",
                            located_value.value
                        ),
                    )
                    .with_context("lint.severity"),
                ),
            }
            continue;
        }

        match rule_name.parse::<LintRule>() {
            Ok(rule) => match located_value.value.parse::<SeverityLevel>() {
                Ok(level) => {
                    result.insert(rule, level);
                }
                Err(_) => errors.push(
                    ConfigValidationError::new(
                        Some(located_value.source.clone()),
                        format!(
                            "invalid severity '{}' for rule '{}'",
                            located_value.value, rule
                        ),
                    )
                    .with_context("lint.severity"),
                ),
            },
            Err(_) => errors.push(
                ConfigValidationError::new(
                    Some(located_value.source.clone()),
                    format!("unknown lint rule '{}' in lint.severity", rule_name),
                )
                .with_context("lint.severity"),
            ),
        }
    }
    (result, wildcard)
}

fn parse_severity_overrides(
    entries: Vec<Located<LintSeverityOverridePartial>>,
    errors: &mut Vec<ConfigValidationError>,
) -> Vec<LintSeverityOverride> {
    let mut overrides = Vec::new();
    for entry in entries {
        let Located { value, source } = entry;
        let pattern = match Pattern::new(source.clone(), value.path.clone()) {
            Ok(pattern) => pattern,
            Err(err) => {
                errors.push(err.with_context("lint.severity_overrides"));
                continue;
            }
        };

        let matcher = pattern.glob().compile_matcher();
        let mut rules = HashMap::new();
        let mut wildcard = None;

        if value.rules.is_empty() {
            errors.push(
                ConfigValidationError::new(
                    Some(source.clone()),
                    format!(
                        "lint.severity_overrides entry for pattern '{}' must specify at least one rule",
                        pattern.original()
                    ),
                )
                .with_context("lint.severity_overrides"),
            );
            continue;
        }

        for (rule_name, severity_value) in value.rules {
            if rule_name == "*" {
                match severity_value.parse::<SeverityLevel>() {
                    Ok(level) => {
                        if wildcard.is_some() {
                            errors.push(
                                ConfigValidationError::new(
                                    Some(source.clone()),
                                    format!(
                                "duplicate wildcard entry in lint.severity_overrides for pattern '{}'",
                                        pattern.original()
                                    ),
                                )
                                .with_context("lint.severity_overrides"),
                            );
                        } else {
                            wildcard = Some(level);
                        }
                    }
                    Err(_) => errors.push(
                        ConfigValidationError::new(
                            Some(source.clone()),
                            format!(
                                "invalid severity '{}' for wildcard in lint.severity_overrides pattern '{}'",
                                severity_value,
                                pattern.original()
                            ),
                        )
                        .with_context("lint.severity_overrides"),
                    ),
                }
                continue;
            }

            match rule_name.parse::<LintRule>() {
                Ok(rule) => match severity_value.parse::<SeverityLevel>() {
                    Ok(level) => {
                        rules.insert(rule, level);
                    }
                    Err(_) => errors.push(
                        ConfigValidationError::new(
                            Some(source.clone()),
                            format!(
                                "invalid severity '{}' for rule '{}' in lint.severity_overrides pattern '{}'",
                                severity_value,
                                rule_name,
                                pattern.original()
                            ),
                        )
                        .with_context("lint.severity_overrides"),
                    ),
                },
                Err(_) => errors.push(
                    ConfigValidationError::new(
                        Some(source.clone()),
                        format!(
                                "unknown lint rule '{}' in lint.severity_overrides pattern '{}'",
                            rule_name,
                            pattern.original()
                        ),
                    )
                    .with_context("lint.severity_overrides"),
                ),
            }
        }

        if rules.is_empty() && wildcard.is_none() {
            errors.push(
                ConfigValidationError::new(
                    Some(source.clone()),
                    format!(
                        "lint.severity_overrides pattern '{}' produced no recognised rules",
                        pattern.original()
                    ),
                )
                .with_context("lint.severity_overrides"),
            );
            continue;
        }

        overrides.push(LintSeverityOverride {
            path: pattern,
            matcher,
            rules,
            wildcard,
            source,
        });
    }
    overrides
}

fn parse_ignore_list(
    entries: Vec<Located<LintIgnorePartial>>,
    errors: &mut Vec<ConfigValidationError>,
) -> Vec<LintIgnore> {
    let mut result = Vec::new();
    for entry in entries {
        let Located { value, source } = entry;
        let pattern = match Pattern::new(source.clone(), value.path.clone()) {
            Ok(pattern) => pattern,
            Err(err) => {
                errors.push(err.with_context("lint.ignore"));
                continue;
            }
        };
        let matcher = pattern.glob().compile_matcher();

        if value.rules.is_empty() {
            errors.push(
                ConfigValidationError::new(
                    Some(source.clone()),
                    format!(
                        "lint.ignore entry for pattern '{}' must specify at least one rule",
                        pattern.original()
                    ),
                )
                .with_context("lint.ignore"),
            );
            continue;
        }

        let mut all_rules = false;
        let mut rules = Vec::new();
        for rule_name in value.rules {
            if rule_name == "*" {
                all_rules = true;
                continue;
            }
            match rule_name.parse::<LintRule>() {
                Ok(rule) => rules.push(rule),
                Err(_) => errors.push(
                    ConfigValidationError::new(
                        Some(source.clone()),
                        format!(
                            "unknown lint rule '{}' in lint.ignore entry for pattern '{}'",
                            rule_name,
                            pattern.original()
                        ),
                    )
                    .with_context("lint.ignore"),
                ),
            }
        }

        if !all_rules && rules.is_empty() {
            errors.push(
                ConfigValidationError::new(
                    Some(source.clone()),
                    format!(
                        "lint.ignore entry for pattern '{}' produced no recognised rules",
                        pattern.original()
                    ),
                )
                .with_context("lint.ignore"),
            );
            continue;
        }

        let rules = if all_rules {
            LintIgnoreRules::All
        } else {
            LintIgnoreRules::Specific(rules)
        };

        result.push(LintIgnore {
            path: pattern,
            matcher,
            rules,
            source,
        });
    }
    result
}

#[derive(Clone, Debug)]
struct ResolvedConfig {
    project: ProjectSettings,
    catalog: CatalogSettings,
    lint: LintSettings,
    schemas: SchemaSettings,
}

/// Container for validation failures, formatted as a bullet list.
#[derive(Debug)]
pub struct ConfigValidationErrors(pub Vec<ConfigValidationError>);

impl fmt::Display for ConfigValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (idx, err) in self.0.iter().enumerate() {
            if idx > 0 {
                writeln!(f)?;
            }
            write!(f, "- {err}")?;
        }
        Ok(())
    }
}

impl ConfigValidationErrors {
    pub fn iter(&self) -> impl Iterator<Item = &ConfigValidationError> {
        self.0.iter()
    }
}

/// Validation failure with optional provenance.
#[derive(Clone, Debug)]
pub struct ConfigValidationError {
    pub source: Option<ConfigSource>,
    pub message: String,
    pub context: Option<String>,
}

impl ConfigValidationError {
    fn new(source: Option<ConfigSource>, message: String) -> Self {
        ConfigValidationError {
            source,
            message,
            context: None,
        }
    }

    fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }
}

impl fmt::Display for ConfigValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(context) = &self.context {
            write!(f, "{}: {}", context, self.message)?;
        } else {
            write!(f, "{}", self.message)?;
        }
        if let Some(source) = &self.source {
            write!(f, " ({})", source.describe())?;
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct RawConfig {
    #[serde(default)]
    project: Option<RawProject>,
    #[serde(default)]
    catalog: Option<RawCatalog>,
    #[serde(default)]
    lint: Option<RawLint>,
    #[serde(default)]
    schemas: Option<HashMap<String, RawSchema>>,
}

impl RawConfig {
    fn into_partial(self, source: ConfigSource) -> PartialConfig {
        PartialConfig {
            project: self
                .project
                .map(|project| project.into_partial(source.clone())),
            catalog: self
                .catalog
                .map(|catalog| catalog.into_partial(source.clone())),
            lint: self.lint.map(|lint| lint.into_partial(source.clone())),
            schemas: self.schemas.map(|schemas| {
                let mut resolved = SchemasPartial::default();
                for (name, schema) in schemas {
                    let partial = schema.into_partial(&name, source.clone());
                    resolved
                        .entries
                        .entry(name)
                        .and_modify(|existing| existing.merge(partial.clone()))
                        .or_insert(partial);
                }
                resolved
            }),
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawProject {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    root: Option<PathBuf>,
    #[serde(default)]
    exclude: Option<Vec<String>>,
}

impl RawProject {
    fn into_partial(self, source: ConfigSource) -> ProjectPartial {
        ProjectPartial {
            name: self.name.map(|value| Located::new(value, source.clone())),
            root: self.root.map(|value| Located::new(value, source.clone())),
            exclude: self
                .exclude
                .map(|value| Located::new(value, source.clone())),
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawCatalog {
    #[serde(default)]
    output: Option<PathBuf>,
    #[serde(default)]
    include_patterns: Option<Vec<String>>,
    #[serde(default)]
    exclude_patterns: Option<Vec<String>>,
}

impl RawCatalog {
    fn into_partial(self, source: ConfigSource) -> CatalogPartial {
        CatalogPartial {
            output: self.output.map(|value| Located::new(value, source.clone())),
            include_patterns: self
                .include_patterns
                .map(|value| Located::new(value, source.clone())),
            exclude_patterns: self
                .exclude_patterns
                .map(|value| Located::new(value, source)),
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawLint {
    #[serde(default)]
    rules: Option<Vec<String>>,
    #[serde(default)]
    max_heading_depth: Option<u8>,
    #[serde(default)]
    toc_start_marker: Option<String>,
    #[serde(default)]
    toc_end_marker: Option<String>,
    #[serde(default)]
    severity: HashMap<String, String>,
    #[serde(default)]
    ignore: Vec<RawLintIgnore>,
    #[serde(default)]
    severity_overrides: Vec<RawLintSeverityOverride>,
}

impl RawLint {
    fn into_partial(self, source: ConfigSource) -> LintPartial {
        let severity = self
            .severity
            .into_iter()
            .map(|(key, value)| (key, Located::new(value, source.clone())))
            .collect();

        let ignore = self
            .ignore
            .into_iter()
            .map(|entry| {
                Located::new(
                    LintIgnorePartial {
                        path: entry.path,
                        rules: entry.rules.unwrap_or_default(),
                    },
                    source.clone(),
                )
            })
            .collect();

        let severity_overrides = self
            .severity_overrides
            .into_iter()
            .map(|entry| {
                Located::new(
                    LintSeverityOverridePartial {
                        path: entry.path,
                        rules: entry.rules,
                    },
                    source.clone(),
                )
            })
            .collect();

        LintPartial {
            rules: self.rules.map(|value| Located::new(value, source.clone())),
            max_heading_depth: self
                .max_heading_depth
                .map(|value| Located::new(value, source.clone())),
            toc_start_marker: self
                .toc_start_marker
                .map(|value| Located::new(value, source.clone())),
            toc_end_marker: self
                .toc_end_marker
                .map(|value| Located::new(value, source.clone())),
            severity,
            ignore,
            severity_overrides,
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawSchema {
    #[serde(default)]
    patterns: Option<Vec<String>>,
    #[serde(default)]
    required_sections: Option<Vec<String>>,
    #[serde(default)]
    allow_additional: Option<bool>,
    #[serde(default)]
    allow_empty: Option<bool>,
    #[serde(default)]
    min_sections: Option<u32>,
    #[serde(default)]
    min_heading_level: Option<u8>,
    #[serde(default)]
    max_heading_level: Option<u8>,
    #[serde(default)]
    require_top_level_heading: Option<bool>,
}

impl RawSchema {
    fn into_partial(self, _name: &str, source: ConfigSource) -> SchemaDefinitionPartial {
        let mut partial = SchemaDefinitionPartial {
            source: Some(source.clone()),
            ..SchemaDefinitionPartial::default()
        };
        if let Some(patterns) = self.patterns {
            partial.patterns = Some(Located::new(patterns, source.clone()));
        }
        if let Some(required_sections) = self.required_sections {
            partial.required_sections = Some(Located::new(required_sections, source.clone()));
        }
        if let Some(allow_additional) = self.allow_additional {
            partial.allow_additional = Some(Located::new(allow_additional, source.clone()));
        }
        if let Some(allow_empty) = self.allow_empty {
            partial.allow_empty = Some(Located::new(allow_empty, source.clone()));
        }
        if let Some(min_sections) = self.min_sections {
            partial.min_sections = Some(Located::new(min_sections, source.clone()));
        }
        if let Some(min_heading_level) = self.min_heading_level {
            partial.min_heading_level = Some(Located::new(min_heading_level, source.clone()));
        }
        if let Some(max_heading_level) = self.max_heading_level {
            partial.max_heading_level = Some(Located::new(max_heading_level, source.clone()));
        }
        if let Some(require_top_level_heading) = self.require_top_level_heading {
            partial.require_top_level_heading =
                Some(Located::new(require_top_level_heading, source));
        }
        partial
    }
}

#[derive(Debug, Deserialize)]
struct RawLintIgnore {
    path: String,
    #[serde(default)]
    rules: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct RawLintSeverityOverride {
    path: String,
    #[serde(default)]
    rules: HashMap<String, String>,
}
