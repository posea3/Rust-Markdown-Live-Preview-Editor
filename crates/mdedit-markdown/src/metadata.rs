#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntaxAttribute {
    name: String,
    value: Option<String>,
}

impl SyntaxAttribute {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }

    pub(crate) fn new(name: String, value: Option<String>) -> Self {
        Self { name, value }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SyntaxBlockQuoteKind {
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SyntaxCodeBlockKind {
    Indented,
    Fenced { info: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SyntaxTableAlignment {
    None,
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SyntaxLinkType {
    Inline,
    Reference,
    ReferenceUnknown,
    Collapsed,
    CollapsedUnknown,
    Shortcut,
    ShortcutUnknown,
    Autolink,
    Email,
    WikiLink { has_alias: bool },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntaxLinkMetadata {
    kind: SyntaxLinkType,
    destination: String,
    title: String,
    reference: String,
}

impl SyntaxLinkMetadata {
    #[must_use]
    pub const fn kind(&self) -> SyntaxLinkType {
        self.kind
    }

    #[must_use]
    pub fn destination(&self) -> &str {
        &self.destination
    }

    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    #[must_use]
    pub fn reference(&self) -> &str {
        &self.reference
    }

    pub(crate) fn new(
        kind: SyntaxLinkType,
        destination: String,
        title: String,
        reference: String,
    ) -> Self {
        Self {
            kind,
            destination,
            title,
            reference,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SyntaxMetadataBlockKind {
    Yaml,
    Pluses,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum SyntaxMetadata {
    #[default]
    None,
    Heading {
        id: Option<String>,
        classes: Vec<String>,
        attributes: Vec<SyntaxAttribute>,
    },
    BlockQuote {
        kind: Option<SyntaxBlockQuoteKind>,
    },
    CodeBlock {
        kind: SyntaxCodeBlockKind,
    },
    List {
        start: Option<u64>,
    },
    FootnoteDefinition {
        label: String,
    },
    Table {
        alignments: Vec<SyntaxTableAlignment>,
    },
    Link(SyntaxLinkMetadata),
    Image(SyntaxLinkMetadata),
    MetadataBlock {
        kind: SyntaxMetadataBlockKind,
    },
    InlineCode {
        content: String,
    },
    InlineMath {
        content: String,
    },
    DisplayMath {
        content: String,
    },
    FootnoteReference {
        label: String,
    },
}
