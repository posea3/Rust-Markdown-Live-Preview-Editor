use mdedit_core::DocumentSnapshot;

use crate::{MarkdownDialect, SyntaxSnapshot};

pub trait MarkdownParser: Send + Sync {
    #[must_use]
    fn parse(
        &self,
        snapshot: &DocumentSnapshot,
        dialect: &MarkdownDialect,
    ) -> SyntaxSnapshot;
}
