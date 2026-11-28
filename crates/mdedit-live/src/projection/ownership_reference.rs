// Frozen brute-force builder/query reference from parent 5cee822. Test-only.
use super::*;

pub(super) fn build_reference(
    document: &DocumentSnapshot,
    syntax: &SyntaxSnapshot,
    blocks: &BlockSnapshot,
    delimiters: &DelimiterSnapshot,
    extensions: Option<&ExtensionSnapshot>,
    policy: RevealPolicy,
    context: &RevealContext,
) -> Result<Projection, ProjectionBuildError> {
    validate_inputs(document, syntax, blocks, delimiters)?;
    if let Some(extensions) = extensions {
        validate_extension_input(document, extensions)?;
    }
    let source = document.text();

    if matches!(syntax.status(), ParseStatus::RawFallback(_)) {
        return Projection::raw(
            document.revision(),
            syntax.source_len(),
            source,
            blocks,
            syntax,
            ProjectionFallbackReason::SyntaxRawFallback,
        );
    }

    let reveal_groups = match build_reveal_groups(syntax.root(), delimiters, policy, context) {
        Ok(groups) => groups,
        Err(ProjectionFallbackReason::UnownedDelimiter) => {
            return Projection::raw(
                document.revision(),
                syntax.source_len(),
                source,
                blocks,
                syntax,
                ProjectionFallbackReason::UnownedDelimiter,
            );
        }
        Err(reason) => {
            return Projection::raw(
                document.revision(),
                syntax.source_len(),
                source,
                blocks,
                syntax,
                reason,
            );
        }
    };

    let mut concealed = reveal_groups
        .iter()
        .filter(|group| !group.revealed)
        .flat_map(|group| {
            group.delimiters.iter().map(|delimiter| ConcealSpan {
                kind: delimiter.kind(),
                source_range: delimiter.range(),
                group: group.id,
            })
        })
        .collect::<Vec<_>>();

    let mut structural_padding = build_structural_padding(&source, &reveal_groups)?;
    let mut widget_candidates = build_widget_candidates(&source, syntax.root(), policy, context)?;
    if let Some(extensions) = extensions {
        widget_candidates.extend(build_extension_widget_candidates(
            &source,
            syntax.root(),
            extensions,
            policy,
            context,
        )?);
        widget_candidates = normalize_widget_candidate_ownership(widget_candidates);
    }
    concealed.retain(|span| !shadowed_by_widget_owner(&widget_candidates, span.source_range));
    structural_padding
        .retain(|span| !shadowed_by_widget_owner(&widget_candidates, span.source_range));

    if has_overlapping_projection_ranges(&concealed, &structural_padding, &widget_candidates) {
        return Projection::raw(
            document.revision(),
            syntax.source_len(),
            source,
            blocks,
            syntax,
            ProjectionFallbackReason::OverlappingConcealment,
        );
    }

    let omitted_ranges = concealed
        .iter()
        .map(|span| span.source_range)
        .chain(structural_padding.iter().map(|span| span.source_range))
        .chain(
            widget_candidates
                .iter()
                .filter(|widget| !widget.active)
                .map(|widget| widget.conceal_range),
        )
        .collect();
    let map = ProjectionMap::from_concealed(syntax.source_len(), omitted_ranges)?;
    let text = materialize_projection(&source, map.concealed_ranges());
    let projected_blocks = project_blocks(blocks, &map)?;
    let styles = project_styles(syntax.root(), &map)?;
    let widgets = project_widgets(&widget_candidates, &map)?;

    Ok(Projection {
        revision: syntax.revision(),
        source_len: syntax.source_len(),
        text,
        status: ProjectionStatus::Projected,
        map,
        blocks: projected_blocks,
        styles,
        widgets,
        reveal_groups,
        concealed,
        structural_padding,
    })
}

fn build_reveal_groups(
    root: &SyntaxNode,
    delimiters: &DelimiterSnapshot,
    policy: RevealPolicy,
    context: &RevealContext,
) -> Result<Vec<RevealGroup>, ProjectionFallbackReason> {
    let mut grouped: BTreeMap<RevealGroupKey, PendingRevealGroup> = BTreeMap::new();

    for delimiter in delimiters.spans() {
        let Some(owner_range) = smallest_owner_range(root, delimiter.owner(), delimiter.range())
        else {
            return Err(ProjectionFallbackReason::UnownedDelimiter);
        };
        let key = (
            owner_range.start().get(),
            owner_range.end().get(),
            syntax_kind_order(delimiter.owner()),
        );
        grouped
            .entry(key)
            .or_insert_with(|| (delimiter.owner(), owner_range, Vec::new()))
            .2
            .push(*delimiter);
    }

    let mut groups = grouped
        .into_values()
        .enumerate()
        .map(|(index, (owner, source_range, mut delimiters))| {
            delimiters.sort_by_key(|delimiter| {
                (
                    delimiter.range().start().get(),
                    delimiter.range().end().get(),
                )
            });
            let id = RevealGroupId(
                u32::try_from(index + 1)
                    .map_err(|_| ProjectionFallbackReason::OverlappingConcealment)?,
            );
            Ok(RevealGroup {
                id,
                owner,
                source_range,
                delimiters,
                revealed: policy == RevealPolicy::SourceVisible,
            })
        })
        .collect::<Result<Vec<_>, ProjectionFallbackReason>>()?;

    if policy == RevealPolicy::ConcealInactive {
        apply_reveal_context(&mut groups, context);
    }

    Ok(groups)
}

pub(super) fn smallest_owner_range(
    node: &SyntaxNode,
    owner: SyntaxKind,
    delimiter: TextRange,
) -> Option<TextRange> {
    let mut best = None;
    smallest_owner_range_into(node, owner, delimiter, &mut best);
    best
}

fn smallest_owner_range_into(
    node: &SyntaxNode,
    owner: SyntaxKind,
    delimiter: TextRange,
    best: &mut Option<TextRange>,
) {
    if !range_contains(node.range(), delimiter) {
        return;
    }

    if node.kind() == owner && best.is_none_or(|current| node.range().len() < current.len()) {
        *best = Some(node.range());
    }

    for child in node.children() {
        smallest_owner_range_into(child, owner, delimiter, best);
    }
}

fn shadowed_by_widget_owner(widgets: &[WidgetCandidate], range: TextRange) -> bool {
    widgets.iter().any(|widget| {
        (!widget.active && range_contains(widget.conceal_range, range))
            || (matches!(widget.kind, WidgetKind::Callout | WidgetKind::CustomEmbed)
                && range_contains(widget.source_range, range))
    })
}

fn build_extension_widget_candidates(
    source: &str,
    root: &SyntaxNode,
    extensions: &ExtensionSnapshot,
    policy: RevealPolicy,
    context: &RevealContext,
) -> Result<Vec<WidgetCandidate>, ProjectionBuildError> {
    let mut widgets = Vec::new();

    for extension in extensions.matches() {
        if extension.kind().extension() != OBSIDIAN_EXTENSION_ID {
            continue;
        }
        match extension.kind().name() {
            OBSIDIAN_CALLOUT => {
                let marker_range = extension.range();
                let Some(blockquote) =
                    smallest_containing_node(root, marker_range, SyntaxKind::BlockQuote)
                else {
                    continue;
                };
                let source_range = extend_range_to_line_ending(source, blockquote.range())?;
                let active = policy == RevealPolicy::SourceVisible
                    || context_touches_range(context, source_range);
                let callout_type = extension.attribute("type").unwrap_or("note").to_owned();
                let title = extension.attribute("title").unwrap_or("").to_owned();
                let fold = extension
                    .attribute("fold")
                    .and_then(|value| value.chars().next());
                let body = callout_body_visible_text(source, blockquote, source_range);

                widgets.push(WidgetCandidate {
                    kind: WidgetKind::Callout,
                    payload: WidgetPayload::Callout(CalloutWidgetData {
                        callout_type,
                        title,
                        fold,
                        body,
                    }),
                    source_range,
                    conceal_range: source_range,
                    active,
                });
            }
            OBSIDIAN_EMBED if image_is_standalone(source, extension.range()) => {
                let source_range = extension.range();
                let active = policy == RevealPolicy::SourceVisible
                    || context_touches_range(context, source_range);
                widgets.push(WidgetCandidate {
                    kind: WidgetKind::CustomEmbed,
                    payload: WidgetPayload::CustomEmbed(CustomEmbedWidgetData {
                        target: extension.attribute("target").unwrap_or("").to_owned(),
                        path: extension.attribute("path").unwrap_or("").to_owned(),
                        fragment: extension.attribute("fragment").map(str::to_owned),
                        display: extension.attribute("display").map(str::to_owned),
                    }),
                    source_range,
                    conceal_range: source_range,
                    active,
                });
            }
            _ => {}
        }
    }

    widgets.sort_by_key(|widget| {
        (
            widget.source_range.start().get(),
            widget.source_range.end().get(),
        )
    });
    Ok(widgets)
}

fn normalize_widget_candidate_ownership(mut widgets: Vec<WidgetCandidate>) -> Vec<WidgetCandidate> {
    let extension_owner_ranges = widgets
        .iter()
        .filter(|widget| matches!(widget.kind, WidgetKind::Callout | WidgetKind::CustomEmbed))
        .map(|widget| widget.source_range)
        .collect::<Vec<_>>();

    widgets.retain(|widget| {
        if matches!(widget.kind, WidgetKind::Callout | WidgetKind::CustomEmbed) {
            return true;
        }

        !extension_owner_ranges
            .iter()
            .any(|range| range_contains(*range, widget.source_range))
    });
    widgets.sort_by_key(|widget| {
        (
            widget.source_range.start().get(),
            widget.source_range.end().get(),
        )
    });
    widgets
}

fn smallest_containing_node(
    node: &SyntaxNode,
    range: TextRange,
    kind: SyntaxKind,
) -> Option<&SyntaxNode> {
    let mut best = None;
    if node.kind() == kind && range_contains(node.range(), range) {
        best = Some(node);
    }

    for child in node.children() {
        if !range_contains(child.range(), range) {
            continue;
        }
        if let Some(candidate) = smallest_containing_node(child, range, kind) {
            best = match best {
                Some(existing) if existing.range().len() <= candidate.range().len() => {
                    Some(existing)
                }
                _ => Some(candidate),
            };
        }
    }
    best
}
