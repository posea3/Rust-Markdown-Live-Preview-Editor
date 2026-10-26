# Live Preview Acceptance

Use this document to validate Phase 5 projection, reveal, navigation, IME, reflow, and accessibility behavior.

## Conceal and reveal

Inactive syntax below should hide only proven Markdown markers and structural padding.

**strong text**
*emphasis text*
~~strikethrough text~~
`inline code`
[linked text](https://example.com)
<https://example.com>

**outer *inner* tail**

***combined emphasis and strong***

Heading with closing markers ###

Setext heading
--------------

> Block quote
> > Nested block quote

- list item
-   padded list item
1. ordered item
-     four-space content indentation must remain represented

## Unicode navigation

한글 입력과 이동
日本語の入力と移動
中文输入与移动
Emoji 👨‍👩‍👧‍👦 and combining é stay on grapheme boundaries.
English العربية English

## Selection and reflow

Select from **before this strong text** across *this emphasis* and into plain text.

Move the caret repeatedly between plain text and the syntax above. Reveal/conceal must not cause the viewport to jump unexpectedly.

## IME scratch

Korean:
Japanese:
Chinese:

Type after each label with the matching native IME. Preedit must remain visible without becoming canonical source until commit.

## Accessibility scratch

Screen readers should expose the projected Live Preview text rather than hidden Markdown marker bytes.

Edit this sentence through Narrator or VoiceOver selection/replacement actions.


## Phase 6 widget scratch

The horizontal rule below should render as a native line while inactive. Moving the caret onto its line should reveal the Markdown source for editing.

Above the rule.

---

Below the rule.


### Task checkbox scratch

- [ ] unchecked task
- [x] checked task

The markers above should render as native checkboxes while inactive. Clicking a checkbox should toggle only its canonical Markdown marker and remain one undo/redo step.


### Image widget scratch

The standalone image below should collapse to a native preview while inactive.

![Phase 6 fixture](phase-06-image.png "local fixture")

This sentence must stay below the full reserved image block and must never overlap the image during decode, reveal/conceal, resize, or scroll.

Inline image source remains editable for now: prefix ![inline](phase-06-image.png) suffix


### Math widget scratch

Display math should render as a native block while inactive:

$$\frac{a}{b} + \sqrt{x} = \sum_{i=1}^{n} i$$

This sentence must remain below the rendered math block and must not overlap it during reveal/conceal or resize.

Inline math remains editable source for now: prefix $x^2 + y^2$ suffix

Invalid display math should fall back safely without crashing:

$$\definitely_unknown_command{x}$$
