pub mod diff;
pub mod engine;
pub mod error;
pub mod fs;
pub mod payload;
pub mod section;

pub use engine::{
    apply_edit, EditOptions, EditOutcome, EditRequest, InsertOptions, Operation, ReplaceOptions,
};
pub use error::{EditError, ExitCode};
pub use markdown_extract::{HeadingKind, MarkdownHeading, SectionSpan};
pub use payload::PayloadSource;
pub use section::{MatchedSection, SectionEdit, SectionTree};
