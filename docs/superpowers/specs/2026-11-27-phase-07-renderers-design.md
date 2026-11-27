# Phase 7 Slice 05 — Host renderer dispatch boundary

Continue above Slice 04 (`00de68e`, PR #17). PRs #16/#17 remain separate reviewable changes; this slice does not merge them.

## Boundary and alternatives

Native GPU, font, resource, layout, clipping, hit-testing, and painting types belong to the application view. A generic engine renderer that owned those resources would couple `mdedit-live` to a UI stack. Arbitrary extension-to-widget conversion would also introduce new concealment and ownership rules. The implementation choice is a small host-owned dispatch registry: it routes existing immutable widget descriptors or qualified extension matches to application callbacks and returns application-defined output/errors.

The registry does not alter parsing, projection, source, widget geometry, or reveal policy. Registering a foreign extension handler does not conceal its source or turn it into an Obsidian widget. Native views may keep their existing direct dispatch code.

## Public contract

- `HostRenderKey` distinguishes `Widget(WidgetKind)` from `Extension { extension: ExtensionId, name: &'static str }`. Extension names are fully qualified; built-in and extension keys cannot alias.
- `HostRenderInput<'a>` borrows either a `ProjectedWidget` or `ExtensionMatch`, derives its own key, and exposes its canonical source range. Callers supply current validated projection/scanner data; the registry is not another document/configuration validator.
- `HostRendererRegistry<Context, Output, Error>` stores host callbacks `FnMut(&mut Context, HostRenderInput<'_>) -> Result<Output, Error>`. Callbacks may own view-thread-only state; no Send/Sync constraint is imposed. Output is host-defined, for example paint commands or editor/host actions.
- `try_register` rejects duplicate keys without replacing the original handler. There is no implicit handler replacement or global registry.
- `render` derives the dispatch key from its input. Missing handlers return `HostRenderError::Unregistered { key, source_range }`; callback failures return `RendererFailed { key, source_range, error }` while preserving the typed cause. Panics propagate and callback side effects are not rolled back.
- `check_widget_support(&Projection)` checks every projected widget for a registered handler before painting and invokes no callbacks. It checks capability, not callback success or geometry.
- A host encountering an unsupported concealed widget must reveal its source and rebuild/reflow projection, or select `RevealPolicy::SourceVisible` for the frame. Merely skipping the missing renderer would leave concealed text blank. For optional foreign metadata decorations, the underlying source remains governed by existing projection rules.
- Rendering context should contain view resources; source edits travel through host/editor actions after rendering. The registry never receives a mutable document. Hosts own renderer/layout cache invalidation and avoid double-rendering built-in extension metadata already represented by projected widgets.

## Scope and verification

Rust 1.90, standard library only, no new dependency, existing public APIs unchanged. No wgpu/winit/font types or new native widget are added. Native GUI/IME acceptance is not rerun or inferred from descriptor tests.

Public tests cover payload/action/range routing, qualified extension namespaces, built-in versus extension key isolation, duplicate preservation, missing-handler errors and callback-free preflight with source-visible fallback, typed failures, stateful non-Send callbacks, panic propagation, and unchanged foreign-source projection. A runnable host example demonstrates dispatch and preflight fallback. Mutation checks remove duplicate protection, qualification, or support checks to prove relevant regressions fail. Complete workspace verification, independent review, and current-head three-OS CI precede completion. The remaining Phase 7 gate is overall compatibility/integration review, not a frozen 1.0 declaration.
