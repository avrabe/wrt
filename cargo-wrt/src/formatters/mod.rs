//! Output formatters for cargo-wrt
//!
//! Provides HTML, Markdown, and other specialized output formats
//! for cargo-wrt command results and reports.

pub mod html;
pub mod markdown;
pub mod templates;

pub use html::{HtmlFormatter, HtmlReportGenerator};
pub use markdown::{create_github_pr_comment, MarkdownFormatter, MarkdownReportGenerator};
pub use templates::{
    DocumentationReportTemplate, RequirementsMatrixTemplate, SafetyReportTemplate,
};
