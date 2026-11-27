#[path = "support/scanners.rs"]
mod scanners;

use std::error::Error;

use mdedit_core::Document;
use mdedit_markdown::{
    ExtensionCandidate, ExtensionError, ExtensionId, ExtensionScanContext, ExtensionScanError,
    ExtensionSet, FallibleSyntaxExtension, MarkdownDialect, MarkdownParser, PulldownCmarkParser,
    SyntaxExtension,
};
use scanners::{IssueScanner, ReferenceScanner};

struct AvailableIssues {
    available: bool,
}

impl FallibleSyntaxExtension for AvailableIssues {
    fn id(&self) -> ExtensionId {
        IssueScanner.id()
    }

    fn try_scan(
        &self,
        context: &ExtensionScanContext<'_>,
    ) -> Result<Vec<ExtensionCandidate>, ExtensionScanError> {
        if self.available {
            Ok(IssueScanner.scan(context))
        } else {
            Err(ExtensionScanError::new("issue index unavailable"))
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let document = Document::new("**한글** {{ref:설계 노트}} #42")?.snapshot();
    let syntax = PulldownCmarkParser.parse(&document, &MarkdownDialect::gfm());

    for available in [true, false] {
        let extensions = ExtensionSet::new()
            .with_extension(ReferenceScanner)
            .with_fallible_extension(AvailableIssues { available });
        match extensions.scan(&document, &syntax) {
            Ok(snapshot) => println!("complete: {} matches", snapshot.matches().len()),
            Err(ExtensionError::ScannerFailed { extension, error }) => {
                println!("scanner '{}' failed: {error}", extension.as_str());
                // Host policy: present canonical source rather than partial
                // metadata. A native host would select source-visible view.
                println!("source-visible fallback: {}", document.text());
            }
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}
