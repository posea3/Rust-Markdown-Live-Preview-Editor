use mdedit_markdown::{ExtensionCandidate, ExtensionId, ExtensionScanContext, SyntaxExtension};

pub struct ReferenceScanner;

impl SyntaxExtension for ReferenceScanner {
    fn id(&self) -> ExtensionId {
        ExtensionId::new("example.references")
    }

    fn scan(&self, context: &ExtensionScanContext<'_>) -> Vec<ExtensionCandidate> {
        let source = context.source();
        source
            .match_indices("{{ref:")
            .filter_map(|(start, prefix)| {
                let target_start = start + prefix.len();
                let target_end = target_start + source[target_start..].find("}}")?;
                let target = source[target_start..target_end].trim();
                if target.is_empty() || target.contains(['\r', '\n']) {
                    return None;
                }
                let range = context.source_range(start, target_end + 2)?;
                Some(ExtensionCandidate::new("reference", range).with_attribute("target", target))
            })
            .collect()
    }
}

pub struct IssueScanner;

impl SyntaxExtension for IssueScanner {
    fn id(&self) -> ExtensionId {
        ExtensionId::new("example.issues")
    }

    fn scan(&self, context: &ExtensionScanContext<'_>) -> Vec<ExtensionCandidate> {
        let source = context.source();
        source
            .match_indices('#')
            .filter_map(|(start, _)| {
                if source[..start]
                    .chars()
                    .next_back()
                    .is_some_and(|character| !character.is_whitespace())
                {
                    return None;
                }
                let number_start = start + 1;
                let digits = source[number_start..]
                    .bytes()
                    .take_while(u8::is_ascii_digit)
                    .count();
                if digits == 0 {
                    return None;
                }
                let end = number_start + digits;
                let range = context.source_range(start, end)?;
                Some(
                    ExtensionCandidate::new("issue", range)
                        .with_attribute("number", &source[number_start..end]),
                )
            })
            .collect()
    }
}
