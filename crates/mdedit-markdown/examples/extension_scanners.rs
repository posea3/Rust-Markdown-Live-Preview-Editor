#[path = "support/scanners.rs"]
mod scanners;

use std::error::Error;

use mdedit_core::Document;
use mdedit_markdown::{ExtensionSet, MarkdownDialect, MarkdownParser, PulldownCmarkParser};
use scanners::{IssueScanner, ReferenceScanner};

fn main() -> Result<(), Box<dyn Error>> {
    let source = "See {{ref:설계 노트}} and #42.\n\n`{{ref:code stays literal}}`\n";
    let document = Document::new(source)?.snapshot();
    let syntax = PulldownCmarkParser.parse(&document, &MarkdownDialect::gfm());
    let extensions = ExtensionSet::new()
        .with_extension(ReferenceScanner)
        .with_extension(IssueScanner);
    let snapshot = extensions.scan(&document, &syntax)?;

    for matched in snapshot.matches() {
        println!(
            "{}::{} {:?} {:?}",
            matched.kind().extension().as_str(),
            matched.kind().name(),
            matched.range().as_usize_range(),
            &source[matched.range().as_usize_range()],
        );
    }
    for issue in snapshot.issues() {
        println!("suppressed {:?}: {:?}", issue.range(), issue.reason());
    }
    Ok(())
}
