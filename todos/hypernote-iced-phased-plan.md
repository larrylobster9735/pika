---
summary: Phased plan for adding Hypernote rendering to the Iced desktop app without blocking on hypernote-mdx parity work for ordinary markdown
read_when:
  - implementing desktop Hypernote support
  - sequencing Iced work against the unified rendering plan
  - deciding what can land before pulldown-cmark removal
---

# Hypernote Iced Phased Plan

## Problem Statement

Desktop now has markdown rendering through `pulldown-cmark`, but it still does
not have a real Hypernote path.

Today:

1. `pika_core` parses Hypernotes with `hypernote-mdx`
2. core exposes Hypernote data on `ChatMessage.hypernote`
3. iOS and Android branch on `message.hypernote` and render dedicated
   Hypernote UIs
4. desktop does not branch on `message.hypernote`
5. desktop also has no Hypernote action plumbing into
   `AppAction::HypernoteAction`

So the immediate desktop gap is not "replace markdown parsing." The immediate
desktop gap is "add Hypernote rendering and interaction support."

## Acceptance Criteria

This effort is done when all of the following are true:

1. Iced renders Hypernote messages from a typed Pika-owned document model.
2. Desktop does not depend on `ast_json` parsing inside Iced UI code.
3. Hypernote actions from desktop dispatch to `AppAction::HypernoteAction`.
4. Existing ordinary chat markdown can continue using `pulldown-cmark`
   temporarily until later unification work lands.
5. The plan does not depend on upstream `hypernote-mdx` parity work for the
   first phases.

## Constraints

Musts:

1. Prefer a typed Pika-owned render model over raw `hypernote-mdx::Ast`
   traversal in desktop UI code.
2. Keep the first slices narrow enough to land without also replacing ordinary
   markdown rendering.
3. Match the current mobile Hypernote surface before inventing desktop-only
   behavior.
4. Keep ordinary message rendering stable while Hypernote support is being
   added.

Must-nots:

1. Do not block the first Iced Hypernote slice on strikethrough or underscore
   emphasis support in `hypernote-mdx`.
2. Do not add a new long-term JSON boundary just for desktop.
3. Do not remove `pulldown-cmark` in the same slice as initial Hypernote
   rendering.

Preferences:

1. Let desktop be the first real consumer of the typed Hypernote document.
2. Reuse the same typed model later for iOS and Android JSON-removal work.
3. Keep desktop-specific code mostly in the Iced view layer, not in core.

Escalations:

1. If the typed document shape becomes too awkward for Iced rendering, revisit
   the Pika-owned document model before building more desktop UI.
2. If a required Hypernote component turns out to need new core fields, extend
   the typed model rather than falling back to AST JSON.

## Proposed Phases

### Phase 0: Core Typed Hypernote Foundation

Goal:

Add typed Hypernote data in `pika_core` while temporarily keeping legacy JSON
fields for compatibility.

Scope:

1. Add `HypernoteDocument`, `HypernoteNode`, and related enums/records in
   `rust/src/state.rs`.
2. Convert `hypernote-mdx` output into that typed document in a dedicated core
   helper module.
3. Normalize `default_state` into typed form-state data such as
   `default_form_state`.
4. Keep `ast_json` and `default_state` temporarily so mobile stays unbroken.
5. Replace JSON-based declared-action extraction with typed traversal.

Why first:

1. Desktop can consume it immediately.
2. It advances the broader unified-rendering plan directly.
3. It avoids building a desktop-only parser boundary.

Files:

1. `rust/src/state.rs`
2. new `rust/src/hypernote.rs`
3. `rust/src/lib.rs`
4. `rust/src/core/storage.rs`
5. `crates/hypernote-protocol/src/lib.rs`

### Phase 1A: Iced Read-Only Hypernote Rendering

Goal:

Make desktop visibly render Hypernote messages using the typed document, even
before interactive submission is wired.

Scope:

1. Add new `crates/pika-desktop/src/views/hypernote.rs`.
2. Branch on `msg.hypernote` in `message_bubble.rs`, similar to mobile.
3. Render the current supported markdown node surface:
   headings, paragraphs, strong, emphasis, inline code, code blocks, links,
   images, lists, blockquotes, hr, hard break, text.
4. Render the current supported Hypernote JSX surface:
   `Card`, `VStack`, `HStack`, `Heading`, `Body`, `Caption`, `Details`,
   `Summary`, `TextInput`, `ChecklistItem`, `SubmitButton`.
5. Render responder/tally UI where already available on the message.

Why split read-only first:

1. It isolates rendering parity from action plumbing.
2. It gives quick feedback on the typed model and desktop layout behavior.

Non-goals:

1. Do not replace ordinary markdown messages yet.
2. Do not remove `pulldown-cmark`.
3. Do not optimize caching yet.

Files:

1. new `crates/pika-desktop/src/views/hypernote.rs`
2. `crates/pika-desktop/src/views/message_bubble.rs`
3. `crates/pika-desktop/src/views/mod.rs`

### Phase 1B: Iced Interactive Hypernote Actions

Goal:

Make desktop Hypernotes actually usable by wiring submit actions and local form
state.

Scope:

1. Add conversation-level UI messages for Hypernote interaction.
2. Add conversation events that carry `message_id`, `action_name`, and form
   data.
3. Dispatch those events from `screen/home.rs` into
   `AppAction::HypernoteAction`.
4. Manage local form state for `TextInput` and `ChecklistItem`.
5. Preserve current optimistic local submission behavior where reasonable.

Files:

1. `crates/pika-desktop/src/views/conversation.rs`
2. `crates/pika-desktop/src/screen/home.rs`
3. `crates/pika-desktop/src/views/hypernote.rs`
4. `crates/pika-desktop/src/views/message_bubble.rs`

### Phase 2: Desktop Hypernote QA And Parity Tightening

Goal:

Confirm the typed Hypernote path is good enough to trust before broadening it.

Scope:

1. Validate poll-like `SubmitButton` flows.
2. Validate `Details` and `Summary`.
3. Validate code block rendering.
4. Validate `TextInput` and `ChecklistItem`.
5. Validate malformed or unsupported nodes fail soft.

Why before normal-message unification:

1. It keeps renderer debugging local to the Hypernote path.
2. It shrinks the blast radius of the first desktop rollout.

### Phase 3: Route Ordinary Desktop Markdown Through The Typed Path

Goal:

Use the same Rust-owned typed content path for ordinary chat messages, not just
Hypernotes.

Scope:

1. Decide whether normal messages use the exact same document model or a close
   sibling typed content model.
2. Add caching where needed for repeated desktop render.
3. Compare current `pulldown-cmark` behavior against the typed path on real
   message fixtures.

Important note:

This phase can wait for upstream `hypernote-mdx` parity work where needed.

### Phase 4: Remove Desktop pulldown-cmark

Goal:

Delete the desktop-only markdown parser once ordinary messages no longer depend
on it.

Acceptance:

1. Hypernotes still work.
2. Ordinary desktop messages still cover the supported message feature set.
3. No desktop message content still depends on `crates/pika-desktop/src/views/markdown.rs`.

## Suggested File-Level Ownership

### Core foundation

1. `rust/src/state.rs`
2. `rust/src/core/storage.rs`
3. new `rust/src/hypernote.rs`
4. `crates/hypernote-protocol/src/lib.rs`

### Iced view work

1. new `crates/pika-desktop/src/views/hypernote.rs`
2. `crates/pika-desktop/src/views/message_bubble.rs`
3. `crates/pika-desktop/src/views/conversation.rs`
4. `crates/pika-desktop/src/screen/home.rs`

## Evaluation Design

### Core tests

1. `Ast -> HypernoteDocument` conversion tests
2. typed action extraction tests
3. default-form-state normalization tests

### Desktop behavior tests

1. Hypernote renderer fixture coverage for code blocks, lists, links, images,
   `Details`, `Summary`, `TextInput`, `ChecklistItem`, and `SubmitButton`
2. conversation event tests for `HypernoteAction` dispatch
3. malformed-node tests to confirm soft failure behavior

### Manual QA fixtures

1. simple poll Hypernote
2. collapsible `Details`
3. code-block Hypernote
4. form Hypernote with text input
5. checklist Hypernote
6. mixed markdown and JSX content

## Why This Does Not Block On hypernote-mdx Feature Requests

The first Iced phases are about rendering Hypernotes, not replacing the ordinary
desktop markdown parser.

That means the following upstream gaps are acceptable for now:

1. no strikethrough support
2. no underscore emphasis support
3. limited rich inline link labels

Those matter later, when we want to migrate normal desktop chat messages away
from `pulldown-cmark`.

## Recommended First Deliverable

The narrowest high-signal first deliverable is:

1. typed Hypernote document dual-write in core
2. Iced Hypernote renderer for Hypernote messages
3. desktop Hypernote action dispatch
4. no change yet to ordinary markdown messages

If that lands cleanly, the later normal-message unification work becomes much
lower risk.
