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
