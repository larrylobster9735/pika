---
summary: Upstream feature requests for hypernote-mdx so it can eventually replace the desktop pulldown-cmark path for ordinary chat messages
read_when:
  - sending parser/renderer requests to the hypernote-mdx team
  - evaluating parity gaps before removing pulldown-cmark on desktop
  - planning later-stage normal-message unification
---

# hypernote-mdx Feature Requests

## Context

Pika desktop added markdown rendering in PR `#644` on March 15, 2026 using
`pulldown-cmark` plus a hand-written Iced renderer.

We want the longer-term direction to be:

1. Rust-owned parsing through `hypernote-mdx`
2. a Pika-owned typed render model in `pika_core`
3. one shared rendering pipeline across Hypernotes and normal chat messages

These requests are about closing parser gaps so `hypernote-mdx` can support the
later phases of that plan cleanly.

## Important Non-Blocking Note

These requests should **not** block the first phases of the Pika plan.

The first phases can still proceed with:

1. typed Hypernote document work in `pika_core`
2. Iced Hypernote rendering for Hypernote messages
3. desktop Hypernote action plumbing
4. keeping `pulldown-cmark` temporarily for ordinary chat markdown

Most of the requests below matter when we want to route normal chat messages
through the same Rust-owned typed content path and then remove
`pulldown-cmark`.

## Requested Features

### P0: Needed For Strong Desktop Markdown Replacement Parity

#### 1. Strikethrough support

Requested behavior:

1. Parse `~~strikethrough~~`
2. represent it in the AST
3. preserve it in tree serialization / render helpers

Why it matters:

1. Desktop already enables and renders strikethrough in the current
   `pulldown-cmark` path.
2. Without this, switching normal messages to `hypernote-mdx` would be a
   regression.

Current observation:

1. No strikethrough token or node is visible in the current `token.rs` or
   `ast.rs` surface.
2. A local runtime probe left `~~gone~~` as literal text rather than parsed
   content.

#### 2. Underscore emphasis parity

Requested behavior:

1. Parse `_italic_`
2. Parse `__bold__`
3. Treat underscore emphasis with the same basic coverage as `*italic*` and
   `**bold**`

Why it matters:

1. This is common markdown syntax in user-authored text.
2. `pulldown-cmark` already supports it.

Current observation:

1. `hypernote-mdx` appears to support `*` and `**`, but not `_` and `__` as
   inline emphasis.
2. A local runtime probe left `_italics_ and __bold__` as literal text.

#### 3. Rich inline content inside link labels

Requested behavior:

1. Support nested inline nodes inside link labels, not just a single text node
2. Example:

```md
[**bold** label](https://example.com)
```

Why it matters:

1. It is valid markdown/MDX content.
2. It makes the parser more generally useful for unified message rendering.

Current observation:

1. Simple `[label](url)` works.
2. `parse_link()` currently only accepts an optional single text node.
3. A local runtime probe for `[**bold** label](https://example.com)` failed to
   parse as a link.

#### 4. Rich inline content inside image alt text

Requested behavior:

1. Treat image alt text similarly to link labels
2. Allow inline children rather than only one optional text node

Why it matters:

1. It keeps link and image handling structurally consistent.
2. It avoids another narrow special case in the typed render conversion layer.

Current observation:

1. `parse_image()` mirrors the same single-text-node shape as `parse_link()`.

### P1: Useful Structural Improvements

#### 5. Stronger blockquote semantics

Requested behavior:

1. Support richer blockquote structure than one line of inline content at a time
2. Preserve multi-line and multi-block quote structure more naturally

Why it matters:

1. The current desktop pulldown path handles nested blockquote structure better
   than the current `hypernote-mdx` parser shape appears to.
2. This will matter more once normal chat messages move to the shared path.

Current observation:

1. `parse_blockquote()` is line-oriented and stops at newline.
2. Simple chat quotes work; richer quote structure is weaker.

#### 6. Stronger list-item block semantics

Requested behavior:

1. Allow list items to contain richer content than a single inline line
2. Preserve nested or multi-paragraph item structure where practical

Why it matters:

1. Simple chat lists work today.
2. More complex markdown lists would be easier to support later if list items
   can carry richer block structure.

Current observation:

1. `parse_list_item()` currently parses inline content up to newline.

## Suggested Acceptance Tests Upstream

### Parsing fixtures

1. `~~gone~~`
2. `_italics_ and __bold__`
3. `[**bold** label](https://example.com)`
4. `![*alt* text](image.png)`
5. multi-line blockquote fixture
6. multi-line or nested list fixture

### Round-trip / rendering expectations

1. parsed structure should preserve the requested features
2. render helpers should not flatten those constructs into plain text
3. tree serialization should expose the requested nodes in a stable way

## Priority Guidance

If the hypernote-mdx team wants a strict order:

1. strikethrough
2. underscore emphasis
3. rich link labels
4. rich image alt text
5. richer blockquotes
6. richer list items

## Bottom Line

`hypernote-mdx` is already strong enough for early Hypernote-focused phases in
Pika.

What it is **not** yet strong enough for is a drop-in replacement for the full
desktop `pulldown-cmark` message path without some parity work first.
