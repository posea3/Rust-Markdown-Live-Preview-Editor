use std::{error::Error, fmt};

use mdedit_core::TextRange;
use mdedit_markdown::{ExtensionId, ExtensionMatch};

use crate::{ProjectedWidget, Projection, WidgetKind};

/// Identity of a host callback. Built-in widgets and qualified extension kinds
/// occupy separate key spaces, even when their local names are the same.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HostRenderKey {
    Widget(WidgetKind),
    Extension {
        extension: ExtensionId,
        name: &'static str,
    },
}

/// Borrowed, immutable metadata from a current projection or scanner snapshot.
/// The host is responsible for document/configuration freshness. An extension
/// input does not by itself create a widget or conceal canonical source.
#[derive(Clone, Copy, Debug)]
pub enum HostRenderInput<'a> {
    Widget(&'a ProjectedWidget),
    Extension(&'a ExtensionMatch),
}

impl HostRenderInput<'_> {
    #[must_use]
    pub const fn key(self) -> HostRenderKey {
        match self {
            Self::Widget(widget) => HostRenderKey::Widget(widget.kind()),
            Self::Extension(item) => HostRenderKey::Extension {
                extension: item.kind().extension(),
                name: item.kind().name(),
            },
        }
    }

    /// Byte range in the canonical source, not projected layout coordinates.
    #[must_use]
    pub const fn source_range(self) -> TextRange {
        match self {
            Self::Widget(widget) => widget.source_range(),
            Self::Extension(item) => item.range(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostRendererRegistrationError {
    DuplicateKey(HostRenderKey),
}

impl fmt::Display for HostRendererRegistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateKey(key) => {
                write!(formatter, "host renderer already registered: {key:?}")
            }
        }
    }
}

impl Error for HostRendererRegistrationError {}

/// Explicit dispatch failure. Hosts choose fallback/retry behavior; earlier
/// callback side effects are not rolled back and panics are not caught.
#[derive(Debug, PartialEq, Eq)]
pub enum HostRenderError<E> {
    Unregistered {
        key: HostRenderKey,
        source_range: TextRange,
    },
    RendererFailed {
        key: HostRenderKey,
        source_range: TextRange,
        error: E,
    },
}

impl<E: fmt::Display> fmt::Display for HostRenderError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unregistered { key, source_range } => {
                write!(
                    formatter,
                    "no host renderer for {key:?} at {source_range:?}"
                )
            }
            Self::RendererFailed {
                key,
                source_range,
                error,
            } => {
                write!(
                    formatter,
                    "host renderer {key:?} failed at {source_range:?}: {error}"
                )
            }
        }
    }
}

impl<E: Error + 'static> Error for HostRenderError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Unregistered { .. } => None,
            Self::RendererFailed { error, .. } => Some(error),
        }
    }
}

type RenderCallback<C, O, E> =
    dyn for<'input> FnMut(&mut C, HostRenderInput<'input>) -> Result<O, E>;

struct RegisteredRenderer<C, O, E> {
    key: HostRenderKey,
    callback: Box<RenderCallback<C, O, E>>,
}

/// Host-owned callback dispatch without a graphics/window dependency.
///
/// `C` contains view resources; `O` can be paint commands or actions for the
/// host to apply later. The registry does not apply editor transactions or
/// own geometry, hit testing, layout, or renderer cache invalidation. Callbacks
/// may capture view-thread-only state: no Send/Sync bound is imposed.
///
/// Use [`Self::check_widget_support`] before painting concealed widgets. Hosts
/// choose which extension matches to dispatch, avoiding duplicate painting of
/// built-in extension metadata already represented by projected widgets.
pub struct HostRendererRegistry<C, O, E> {
    renderers: Vec<RegisteredRenderer<C, O, E>>,
}

impl<C, O, E> Default for HostRendererRegistry<C, O, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C, O, E> HostRendererRegistry<C, O, E> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            renderers: Vec::new(),
        }
    }

    /// Register one callback. A duplicate key returns an error and preserves
    /// the original handler. This does not change projection or reveal rules.
    pub fn try_register<F>(
        &mut self,
        key: HostRenderKey,
        callback: F,
    ) -> Result<(), HostRendererRegistrationError>
    where
        F: for<'input> FnMut(&mut C, HostRenderInput<'input>) -> Result<O, E> + 'static,
    {
        if self.supports(key) {
            return Err(HostRendererRegistrationError::DuplicateKey(key));
        }
        self.renderers.push(RegisteredRenderer {
            key,
            callback: Box::new(callback),
        });
        Ok(())
    }

    #[must_use]
    pub fn supports(&self, key: HostRenderKey) -> bool {
        self.renderers.iter().any(|renderer| renderer.key == key)
    }

    /// Dispatch exactly one input using its derived, qualified key. An absent
    /// handler is an error; callback errors preserve their typed cause.
    /// Panics and prior host side effects remain the host's responsibility.
    pub fn render(
        &mut self,
        context: &mut C,
        input: HostRenderInput<'_>,
    ) -> Result<O, HostRenderError<E>> {
        let key = input.key();
        let source_range = input.source_range();
        let renderer = self
            .renderers
            .iter_mut()
            .find(|renderer| renderer.key == key)
            .ok_or(HostRenderError::Unregistered { key, source_range })?;
        (renderer.callback)(context, input).map_err(|error| HostRenderError::RendererFailed {
            key,
            source_range,
            error,
        })
    }

    /// Check all projected widget keys before painting, without invoking any
    /// callbacks. Returns the first unsupported widget; successful preflight
    /// guarantees registration only, not callback success or correct geometry.
    ///
    /// An unsupported widget may already conceal source in `projection`.
    /// Rebuild/reflow with its source revealed, or use
    /// [`crate::RevealPolicy::SourceVisible`] for the frame. Skipping its callback
    /// alone leaves a blank concealed range. Callback failures during rendering
    /// need a separate host fallback, including discarding partial paint output.
    pub fn check_widget_support(&self, projection: &Projection) -> Result<(), HostRenderError<E>> {
        for widget in projection.widgets() {
            let key = HostRenderKey::Widget(widget.kind());
            if !self.supports(key) {
                return Err(HostRenderError::Unregistered {
                    key,
                    source_range: widget.source_range(),
                });
            }
        }
        Ok(())
    }
}
