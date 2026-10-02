#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarkdownDialect {
    tables: bool,
    footnotes: bool,
    strikethrough: bool,
    task_lists: bool,
    gfm: bool,
    heading_attributes: bool,
    math: bool,
    wikilinks: bool,
    definition_lists: bool,
    metadata_blocks: bool,
    superscript: bool,
    subscript: bool,
}

impl MarkdownDialect {
    #[must_use]
    pub const fn commonmark() -> Self {
        Self {
            tables: false,
            footnotes: false,
            strikethrough: false,
            task_lists: false,
            gfm: false,
            heading_attributes: false,
            math: false,
            wikilinks: false,
            definition_lists: false,
            metadata_blocks: false,
            superscript: false,
            subscript: false,
        }
    }

    #[must_use]
    pub const fn gfm() -> Self {
        Self {
            tables: true,
            footnotes: false,
            strikethrough: true,
            task_lists: true,
            gfm: true,
            heading_attributes: false,
            math: false,
            wikilinks: false,
            definition_lists: false,
            metadata_blocks: false,
            superscript: false,
            subscript: false,
        }
    }

    #[must_use]
    pub const fn extended() -> Self {
        Self {
            tables: true,
            footnotes: true,
            strikethrough: true,
            task_lists: true,
            gfm: true,
            heading_attributes: true,
            math: true,
            wikilinks: true,
            definition_lists: true,
            metadata_blocks: true,
            superscript: true,
            subscript: true,
        }
    }

    #[must_use]
    pub const fn tables(self) -> bool {
        self.tables
    }

    #[must_use]
    pub const fn footnotes(self) -> bool {
        self.footnotes
    }

    #[must_use]
    pub const fn strikethrough(self) -> bool {
        self.strikethrough
    }

    #[must_use]
    pub const fn task_lists(self) -> bool {
        self.task_lists
    }

    #[must_use]
    pub const fn gfm_extensions(self) -> bool {
        self.gfm
    }

    #[must_use]
    pub const fn heading_attributes(self) -> bool {
        self.heading_attributes
    }

    #[must_use]
    pub const fn math(self) -> bool {
        self.math
    }

    #[must_use]
    pub const fn wikilinks(self) -> bool {
        self.wikilinks
    }

    #[must_use]
    pub const fn definition_lists(self) -> bool {
        self.definition_lists
    }

    #[must_use]
    pub const fn metadata_blocks(self) -> bool {
        self.metadata_blocks
    }

    #[must_use]
    pub const fn superscript(self) -> bool {
        self.superscript
    }

    #[must_use]
    pub const fn subscript(self) -> bool {
        self.subscript
    }
}

impl Default for MarkdownDialect {
    fn default() -> Self {
        Self::commonmark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commonmark_keeps_extensions_disabled() {
        let dialect = MarkdownDialect::commonmark();

        assert!(!dialect.tables());
        assert!(!dialect.strikethrough());
        assert!(!dialect.task_lists());
        assert!(!dialect.wikilinks());
    }

    #[test]
    fn gfm_enables_the_expected_editor_extensions() {
        let dialect = MarkdownDialect::gfm();

        assert!(dialect.tables());
        assert!(dialect.strikethrough());
        assert!(dialect.task_lists());
        assert!(dialect.gfm_extensions());
        assert!(!dialect.math());
    }
}
