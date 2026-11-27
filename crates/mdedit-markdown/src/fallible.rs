use std::{error::Error, fmt};

use crate::{ExtensionCandidate, ExtensionId, ExtensionScanContext, ExtensionScanMode};

/// A scanner-declared failure, distinct from candidate-validation diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtensionScanError {
    message: String,
}

impl ExtensionScanError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ExtensionScanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ExtensionScanError {}

/// Synchronous scanner that can report a recoverable failure to the host.
///
/// Register through `crate::ExtensionSet::try_register_fallible` or its
/// convenience variants. A reported error stops the whole scan; no partial
/// snapshot is returned. Callback panics still propagate and scanner-owned
/// side effects from earlier callbacks are not rolled back.
pub trait FallibleSyntaxExtension: Send + Sync {
    /// Namespace captured at registration, shared with legacy scanner IDs.
    fn id(&self) -> ExtensionId;

    fn scan_mode(&self) -> ExtensionScanMode {
        ExtensionScanMode::MarkdownText
    }

    /// Return candidates for normal validation and merging.
    ///
    /// # Errors
    ///
    /// Return an error when this scanner cannot produce a complete candidate
    /// list. The set reports the registered namespace and cause to the host.
    fn try_scan(
        &self,
        context: &ExtensionScanContext<'_>,
    ) -> Result<Vec<ExtensionCandidate>, ExtensionScanError>;
}
