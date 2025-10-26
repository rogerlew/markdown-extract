use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use globset::GlobMatcher;
use markdown_doc_config::{SchemaDefinition, SchemaSettings};
use markdown_doc_parser::{normalize_heading_text, DocumentSection};

use crate::lines::byte_to_line;

#[derive(Clone)]
pub struct SchemaEngine {
    default: Arc<SchemaModel>,
    definitions: HashMap<String, Arc<SchemaModel>>,
    matchers: Vec<SchemaPatternMatcher>,
}

#[derive(Clone, Debug)]
pub struct SchemaModel {
    name: String,
    allow_additional: bool,
    allow_empty: bool,
    min_sections: Option<u32>,
    min_heading_level: Option<u8>,
    max_heading_level: Option<u8>,
    require_top_level_heading: bool,
    required_sections: Vec<RequiredSection>,
    required_lookup: HashSet<String>,
}

#[derive(Clone, Debug)]
struct RequiredSection {
    display: String,
    normalized: String,
}

#[derive(Clone)]
struct SchemaPatternMatcher {
    matcher: GlobMatcher,
    schema: Arc<SchemaModel>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct SchemaCheck {
    pub schema: Arc<SchemaModel>,
    pub violations: Vec<SchemaViolation>,
}

impl SchemaCheck {
    #[allow(dead_code)]
    pub fn is_success(&self) -> bool {
        self.violations.is_empty()
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct SchemaViolation {
    pub kind: SchemaViolationKind,
    pub message: String,
    pub line: usize,
}

impl SchemaViolation {
    fn new(kind: SchemaViolationKind, line: usize, message: String) -> Self {
        Self {
            kind,
            message,
            line,
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum SchemaViolationKind {
    MissingSection {
        name: String,
        expected_after: Option<String>,
    },
    SectionOrder {
        name: String,
        expected_after: String,
    },
    UnexpectedSection {
        name: String,
    },
    HeadingTooDeep {
        name: String,
        depth: usize,
        max: u8,
    },
    HeadingTooShallow {
        name: String,
        depth: usize,
        min: u8,
    },
    MissingTopLevelHeading,
    EmptyDocument,
    BelowMinimumSections {
        required: u32,
        found: usize,
    },
}

impl SchemaEngine {
    pub fn new(settings: &SchemaSettings) -> Self {
        let mut definitions = HashMap::new();
        for (name, definition) in &settings.schemas {
            let model = Arc::new(SchemaModel::from_definition(definition));
            definitions.insert(name.clone(), model);
        }

        let default = definitions
            .get(&settings.default_schema)
            .cloned()
            .or_else(|| definitions.values().next().cloned())
            .unwrap_or_else(|| Arc::new(SchemaModel::fallback()));

        let mut matchers = Vec::new();
        for pattern in &settings.patterns {
            if let Some(schema) = definitions.get(&pattern.schema) {
                matchers.push(SchemaPatternMatcher {
                    matcher: pattern.matcher.glob().compile_matcher(),
                    schema: schema.clone(),
                });
            }
        }

        SchemaEngine {
            default,
            definitions,
            matchers,
        }
    }

    pub fn schema_by_name(&self, name: &str) -> Option<Arc<SchemaModel>> {
        self.definitions.get(name).cloned()
    }

    pub fn schema_for_path(&self, path: &Path) -> Arc<SchemaModel> {
        for entry in &self.matchers {
            if entry.matcher.is_match(path) {
                return entry.schema.clone();
            }
        }
        self.default.clone()
    }

    pub fn check(
        &self,
        schema: Arc<SchemaModel>,
        sections: &[DocumentSection],
        line_offsets: &[usize],
    ) -> SchemaCheck {
        let violations = schema.evaluate(sections, line_offsets);
        SchemaCheck { schema, violations }
    }
}

impl SchemaModel {
    fn from_definition(definition: &SchemaDefinition) -> Self {
        let required_sections: Vec<RequiredSection> = definition
            .required_sections
            .iter()
            .map(|name| {
                let normalized = normalize_heading_text(name).to_ascii_lowercase();
                RequiredSection {
                    display: name.clone(),
                    normalized,
                }
            })
            .collect();

        let required_lookup = required_sections
            .iter()
            .map(|section| section.normalized.clone())
            .collect();

        SchemaModel {
            name: definition.name.clone(),
            allow_additional: definition.allow_additional,
            allow_empty: definition.allow_empty,
            min_sections: definition.min_sections,
            min_heading_level: definition.min_heading_level,
            max_heading_level: definition.max_heading_level,
            require_top_level_heading: definition.require_top_level_heading.unwrap_or(false),
            required_sections,
            required_lookup,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    fn fallback() -> Self {
        SchemaModel {
            name: "default".into(),
            allow_additional: true,
            allow_empty: false,
            min_sections: None,
            min_heading_level: None,
            max_heading_level: None,
            require_top_level_heading: false,
            required_sections: Vec::new(),
            required_lookup: HashSet::new(),
        }
    }

    #[allow(dead_code)]
    pub fn required_sections(&self) -> impl Iterator<Item = &str> {
        self.required_sections
            .iter()
            .map(|section| section.display.as_str())
    }

    #[allow(dead_code)]
    pub fn allow_additional(&self) -> bool {
        self.allow_additional
    }

    fn evaluate(
        &self,
        sections: &[DocumentSection],
        line_offsets: &[usize],
    ) -> Vec<SchemaViolation> {
        let mut violations = Vec::new();

        if sections.is_empty() {
            if !self.allow_empty {
                violations.push(SchemaViolation::new(
                    SchemaViolationKind::EmptyDocument,
                    0,
                    format!(
                        "Document is empty but schema '{}' disallows empty files",
                        self.name
                    ),
                ));
            }
            return violations;
        }

        let mut has_top_level = false;
        let mut headings = Vec::new();

        for section in sections {
            let depth = section.heading.depth;
            if depth == 1 {
                has_top_level = true;
            }

            if let Some(min_level) = self.min_heading_level {
                if depth < min_level as usize {
                    violations.push(SchemaViolation::new(
                        SchemaViolationKind::HeadingTooShallow {
                            name: section.heading.raw.clone(),
                            depth,
                            min: min_level,
                        },
                        byte_to_line(section.heading.byte_range.start, line_offsets),
                        format!(
                            "Section '{}' depth {} is shallower than minimum heading level {}",
                            section.heading.raw, depth, min_level
                        ),
                    ));
                }
            }

            if let Some(max_level) = self.max_heading_level {
                if depth > max_level as usize {
                    violations.push(SchemaViolation::new(
                        SchemaViolationKind::HeadingTooDeep {
                            name: section.heading.raw.clone(),
                            depth,
                            max: max_level,
                        },
                        byte_to_line(section.heading.byte_range.start, line_offsets),
                        format!(
                            "Section '{}' depth {} exceeds max heading level {}",
                            section.heading.raw, depth, max_level
                        ),
                    ));
                }
            }

            let normalized = section.heading.normalized.to_ascii_lowercase();
            headings.push(HeadingRecord {
                raw: section.heading.raw.clone(),
                normalized,
                depth,
                line: byte_to_line(section.heading.byte_range.start, line_offsets),
            });
        }

        if self.require_top_level_heading && !has_top_level {
            violations.push(SchemaViolation::new(
                SchemaViolationKind::MissingTopLevelHeading,
                0,
                format!(
                    "Schema '{}' requires at least one top-level heading (depth 1)",
                    self.name
                ),
            ));
        }

        let total_sections = headings.len();
        if let Some(min_sections) = self.min_sections {
            if (total_sections as u32) < min_sections {
                violations.push(SchemaViolation::new(
                    SchemaViolationKind::BelowMinimumSections {
                        required: min_sections,
                        found: total_sections,
                    },
                    0,
                    format!(
                        "Document contains {} sections but schema '{}' requires at least {}",
                        total_sections, self.name, min_sections
                    ),
                ));
            }
        }

        let mut positions = Vec::new();
        for requirement in &self.required_sections {
            let position = headings
                .iter()
                .enumerate()
                .find(|(_, heading)| heading.normalized == requirement.normalized)
                .map(|(idx, _)| idx);
            positions.push(position);
        }

        let mut last_present: Option<&RequiredSection> = None;
        for (requirement, position) in self.required_sections.iter().zip(positions.iter()) {
            if position.is_some() {
                last_present = Some(requirement);
            } else {
                let expected_after = last_present.map(|req| req.display.clone());
                violations.push(SchemaViolation::new(
                    SchemaViolationKind::MissingSection {
                        name: requirement.display.clone(),
                        expected_after: expected_after.clone(),
                    },
                    0,
                    match expected_after {
                        Some(prev) => format!(
                            "Missing required section '{}' (expected after '{}')",
                            requirement.display, prev
                        ),
                        None => format!(
                            "Missing required section '{}' at the beginning of the document",
                            requirement.display
                        ),
                    },
                ));
            }
        }

        let mut previous_index: Option<usize> = None;
        let mut previous_requirement: Option<&RequiredSection> = None;
        for (requirement, position) in self.required_sections.iter().zip(positions.iter()) {
            if let Some(index) = position {
                if let Some(prev_index) = previous_index {
                    if *index < prev_index {
                        if let Some(record) = headings.get(*index) {
                            if let Some(prev_req) = previous_requirement {
                                violations.push(SchemaViolation::new(
                                    SchemaViolationKind::SectionOrder {
                                        name: record.raw.clone(),
                                        expected_after: prev_req.display.clone(),
                                    },
                                    record.line,
                                    format!(
                                        "Section '{}' appears before required section '{}'",
                                        record.raw, prev_req.display
                                    ),
                                ));
                            }
                        }
                    }
                }
                previous_index = Some(*index);
                previous_requirement = Some(requirement);
            }
        }

        if !self.allow_additional {
            for record in &headings {
                if !self.required_lookup.contains(&record.normalized) {
                    violations.push(SchemaViolation::new(
                        SchemaViolationKind::UnexpectedSection {
                            name: record.raw.clone(),
                        },
                        record.line,
                        format!(
                            "Unexpected section '{}' not permitted by schema '{}'",
                            record.raw, self.name
                        ),
                    ));
                }
            }
        }

        violations
    }
}

#[allow(dead_code)]
#[derive(Clone)]
struct HeadingRecord {
    raw: String,
    normalized: String,
    depth: usize,
    line: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use markdown_doc_parser::ParserContext;
    use markdown_doc_test_support::test_config;
    use std::path::Path;

    #[test]
    fn evaluate_reports_missing_section() {
        let mut config = test_config();
        if let Some(default_schema) = config.schemas.schemas.get_mut("default") {
            default_schema.required_sections = vec!["Overview".to_string()];
            default_schema.allow_additional = true;
            default_schema.allow_empty = false;
        }

        let engine = SchemaEngine::new(&config.schemas);

        let parser = ParserContext::new(config.clone());
        let contents = "# Title\n";
        let sections = parser.sections_from_str(Path::new("docs/sample.md"), contents);
        let line_offsets = crate::lines::compute_line_offsets(contents);

        let schema = engine.schema_for_path(Path::new("docs/sample.md"));
        let check = engine.check(schema, &sections, &line_offsets);

        assert!(check
            .violations
            .iter()
            .any(|violation| matches!(violation.kind, SchemaViolationKind::MissingSection { .. })));
    }
}
