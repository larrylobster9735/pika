---
summary: Feasibility study for replacing the desktop Iced markdown renderer with hypernote-mdx and for adding Hypernote rendering to desktop as a staged unification step
read_when:
  - evaluating desktop markdown or Hypernote rendering
  - extending the unified Hypernote rendering plan to Iced
  - deciding whether desktop should switch away from pulldown-cmark
---

# Iced Hypernote Feasibility Study

## Problem Statement

Desktop gained markdown rendering in PR `#644` on March 15, 2026 via a new
`pulldown-cmark`-backed renderer in `crates/pika-desktop/src/views/markdown.rs`.
That means Pika now has three separate chat markdown stacks:

1. iOS: `MarkdownUI`
2. Android: `compose-markdown`
3. Desktop: `pulldown-cmark` plus hand-written Iced rendering

At the same time, Hypernotes already parse in Rust through `hypernote-mdx`.
The question is not just "can desktop call `hypernote-mdx`?" It can. The real
question is which desktop-first slice reduces long-term duplication instead of
creating a fourth partially-overlapping rendering path.

## Acceptance Criteria

This study should answer all of the following:

1. Is it technically feasible to replace the new Iced markdown path with
   `hypernote-mdx`?
2. What blocks Hypernote rendering on desktop today?
3. What is the smallest safe first slice that adds desktop Hypernote support
   while still helping the broader unified-rendering plan?
4. What tests would make that slice credible?

## Constraints

Musts:

1. Prefer a Pika-owned typed render model over pushing parser-internal AST
   details into more app code.
2. Keep the first desktop slice landable without requiring full normal-message
   unification in the same change.
3. Avoid deepening the existing JSON AST boundary unless the code is clearly
   temporary.
4. Preserve current desktop markdown behavior for ordinary chat until the new
   path reaches acceptable parity.

Must-nots:

1. Do not swap `pulldown-cmark` for raw `hypernote-mdx::Ast` traversal in Iced
   as a permanent architecture.
2. Do not make desktop the only platform with a bespoke Hypernote model.
3. Do not remove desktop markdown rendering before desktop has a working
   replacement for plain markdown messages.

Preferences:

1. Let desktop become an early consumer of the same typed Hypernote model the
   mobile plan already wants.
2. Use desktop to validate render traversal and interaction handling before
   removing native mobile markdown dependencies.

## Current Reality

### Desktop

1. `crates/pika-desktop/Cargo.toml` now depends on `pulldown-cmark`.
2. `crates/pika-desktop/src/views/markdown.rs` renders a limited subset of
   markdown into Iced widgets.
3. `crates/pika-desktop/src/views/message_bubble.rs` renders
   `MessageSegment::Markdown` and skips `MessageSegment::PikaHtml`.
4. Desktop does not branch on `msg.hypernote` the way iOS and Android do.
5. Desktop conversation/home event plumbing has no Hypernote-specific UI events
   and no dispatch path to `AppAction::HypernoteAction`.

### Core / Hypernote

1. `pika_core` already depends on `hypernote-mdx` from the workspace root.
2. `rust/src/core/storage.rs` parses kind-9467 messages with `hypernote-mdx`
   and stores the result as `HypernoteData.ast_json`.
3. `rust/src/state.rs` still exposes `HypernoteData.ast_json` and
   `default_state`.
4. iOS and Android render Hypernotes from that JSON today.

### Mobile parity reference

1. iOS branches on `message.hypernote` in
   `ios/Sources/Views/MessageBubbleViews.swift`.
2. Android branches on `message.hypernote` in
   `android/app/src/main/java/com/pika/app/ui/screens/ChatScreen.kt`.
3. Both mobile platforms have dedicated Hypernote renderers already.

## Findings

### 1. Replacing desktop markdown with hypernote-mdx is technically feasible

Desktop is Rust. It can call `hypernote_mdx::parse(...)` directly without FFI
or generated bindings.

That is the easy part.

### 2. Raw hypernote-mdx AST is the wrong long-term app boundary

`hypernote-mdx` exposes a real traversable AST, but it is parser-oriented:

1. `Ast` includes token tables and `extra_data`.
2. Traversal depends on parser-specific helpers like `children()`,
   `heading_info()`, `jsx_element()`, and `jsx_attributes()`.
3. That shape is fine inside Rust parsing code, but it is not the clean app
   model we want desktop, iOS, and Android to share.

So the answer is not "no." The answer is "yes, but direct AST consumption is a
bridge, not the destination."

### 3. The current desktop markdown renderer is intentionally narrow

The new Iced renderer is useful, but it is a lightweight stopgap rather than a
strong candidate for a final shared architecture.

Notable limitations:

1. It enables only `Options::ENABLE_STRIKETHROUGH`.
2. Links are styled, but the renderer does not make them actionable.
3. Images, tables, and raw HTML/MDX are not rendered in the current view code.
4. Headings are visually bolded but not mapped to richer desktop-specific
   heading presentation.
5. The code is written around pulldown events, not around a reusable typed
   document model.

This makes desktop a reasonable place to replace later, but not by swapping one
parser dependency for another inside the same file.

### 4. Desktop does not really support Hypernotes today

This is the most important desktop-specific finding.

1. `ChatMessage.hypernote` is populated in core for Hypernote messages.
2. iOS and Android use that field to switch renderers.
3. Desktop ignores that field and continues through the normal message bubble
   path.
4. `message_bubble.rs` contains a comment saying `PikaHtml` or Hypernotes are
   rendered separately, but no desktop Hypernote renderer currently exists.

So desktop's missing feature is larger than "replace the markdown crate." It is
"add the whole Hypernote rendering path."

### 5. Desktop Hypernote support needs action plumbing, not just rendering

The core already supports `AppAction::HypernoteAction`, but desktop UI does not
currently surface it.

That means a real Iced Hypernote implementation also needs:

1. conversation-level UI messages/events for Hypernote actions
2. state for local form interaction where needed
3. `screen/home.rs` dispatch into `AppAction::HypernoteAction`

So the first desktop slice is not only about parsing.

### 6. Desktop is a good first consumer of a typed Hypernote document

This is the strongest case for doing desktop work now.

If core first grows a Pika-owned typed `HypernoteDocument`:

1. desktop can render it immediately
2. desktop can validate traversal and interaction mapping
3. the same model can later replace JSON on iOS and Android
4. the work directly advances the broader unified-rendering plan

That is materially better than adding a desktop-only raw AST renderer.

## Option Review

### Option A: Replace `markdown.rs` with direct hypernote-mdx AST traversal now

Pros:

1. Removes the new desktop-only markdown dependency from this code path.
2. Keeps the work entirely in Rust.

Cons:

1. Still does not solve desktop Hypernote widgets by itself.
2. Couples desktop UI code to parser internals.
3. Does not help iOS or Android migrate off JSON.
4. Likely gets thrown away once a typed document exists.

Verdict:

Not recommended as the first real slice.

### Option B: Add a desktop Hypernote renderer against today's `ast_json`

Pros:

1. Fastest path to visible desktop Hypernote support.
2. Follows the current mobile renderer boundary.

Cons:

1. Deepens the AST JSON boundary the main plan is trying to delete.
2. Duplicates the same temporary debt on another client.
3. Creates more cleanup work later.

Verdict:

Acceptable only as a throwaway spike, not as the preferred implementation path.

### Option C: Add typed Hypernote data in core, then render Hypernotes in Iced from that

Pros:

1. Advances the broader plan directly.
2. Gives desktop Hypernote support without adding new long-term debt.
3. Lets desktop validate the typed model before mobile fully switches.
4. Keeps `pulldown-cmark` in place temporarily for normal chat, which shrinks
   the first slice.

Cons:

1. Slightly larger first PR than a renderer-only desktop change.
2. Requires core model work before the Iced UI lands.

Verdict:

Recommended.

### Option D: Replace all desktop chat rendering with hypernote-mdx in one shot

Pros:

1. Maximum immediate consolidation on desktop.

Cons:

1. Too wide for a first slice.
2. Mixes Hypernote parity, normal markdown parity, interaction plumbing, and
   dependency removal in one change.
3. Harder to test and harder to back out.

Verdict:

Do not start here.

## Recommendation

The right answer is:

1. Yes, desktop can eventually replace the new markdown renderer with the
   `hypernote-mdx` pipeline.
2. No, the first step should not be a direct `pulldown-cmark` to
   `hypernote-mdx::Ast` swap inside `crates/pika-desktop/src/views/markdown.rs`.
3. The best first slice is to add typed Hypernote rendering to desktop through a
   Pika-owned document model, while keeping ordinary markdown messages on the
   current `pulldown-cmark` path temporarily.

In other words:

1. add typed Hypernote data in core
2. add Iced Hypernote rendering against that typed data
3. add desktop Hypernote action plumbing
4. only then consider routing ordinary markdown messages through the same typed
   content path
5. remove `pulldown-cmark` from desktop last

## Proposed Decomposition

### Slice 0A: Core typed Hypernote dual-write

Goal:

Add a typed `HypernoteDocument` and typed default form state in core, while
temporarily keeping `ast_json` and `default_state` for mobile compatibility.

Why this first:

1. It helps desktop immediately.
2. It is already the recommended first move in the broader plan.
3. It avoids writing a new desktop-only parser boundary.

### Slice 0B: Desktop Hypernote renderer over typed data

Goal:

Add `crates/pika-desktop/src/views/hypernote.rs` and branch on
`msg.hypernote` in `message_bubble.rs`, matching the mobile structure.

Likely scope:

1. headings
2. paragraphs and inline styles
3. code blocks
4. blockquotes
5. ordered and unordered lists
6. `Card`, `VStack`, `HStack`
7. `Details` and `Summary`
8. `TextInput`
9. `ChecklistItem`
10. `SubmitButton`
11. responder avatars and tallies

### Slice 0C: Desktop Hypernote action plumbing

Goal:

Add conversation and home-screen event plumbing so Hypernote submit actions
dispatch to `AppAction::HypernoteAction`.

Likely files:

1. `crates/pika-desktop/src/views/conversation.rs`
2. `crates/pika-desktop/src/screen/home.rs`
3. `crates/pika-desktop/src/views/message_bubble.rs`
4. new `crates/pika-desktop/src/views/hypernote.rs`

### Slice 1: Expand typed rendering to normal chat messages

Goal:

Route ordinary markdown messages through Rust-owned typed parsing, with caching
where appropriate.

Desktop note:

This is the point where replacing `crates/pika-desktop/src/views/markdown.rs`
becomes sensible.

### Slice 2: Remove desktop pulldown-cmark

Goal:

Delete the desktop-only markdown parser once ordinary chat messages render
through the typed path with acceptable parity.

## Evaluation Design

### Functional tests

1. Rust tests for `Ast -> HypernoteDocument` conversion.
2. Rust tests for typed submit-action extraction.
3. Rust tests for default-state normalization.
4. Desktop tests for conversation event plumbing:
   Hypernote submit should become `AppAction::HypernoteAction`.

### UI regression fixtures

Desktop should be manually verified against representative Hypernotes:

1. poll-like `SubmitButton` hypernote
2. `Details` and `Summary`
3. code block with language label
4. `ChecklistItem`
5. `TextInput` plus submit
6. mixed markdown and JSX blocks

### Normal markdown safety checks

Before deleting `pulldown-cmark`, verify ordinary desktop messages still cover:

1. emphasis
2. links
3. code blocks
4. blockquotes
5. lists
6. images if required for parity

### Failure-mode tests

1. malformed MDX should fail soft, not panic
2. unknown JSX components should render safely as unsupported or inert content
3. empty or missing default state should not break submission

## Impact on the Existing Unified Rendering Plan

The existing `todos/hypernote-unified-rendering-plan.md` is still directionally
correct, but it now needs one desktop-specific refinement:

1. after core typed dual-write, desktop can be the first renderer consumer of
   the typed Hypernote document
2. desktop Hypernote rendering can land before full normal-message unification
3. desktop markdown dependency removal should happen after that, not before

That means the best desktop-first rollout is:

1. typed Hypernote document in core
2. Iced Hypernote renderer
3. desktop Hypernote action plumbing
4. mobile migration off JSON
5. normal-message unification
6. dependency removals

## Bottom Line

Can the Iced markdown path be replaced with `hypernote-mdx`?

Yes, but the safe and useful path is:

1. do not replace `pulldown-cmark` with raw `hypernote-mdx::Ast` traversal as
   the first step
2. do add typed Hypernote rendering to Iced now
3. use that work as the first concrete stage toward unified rendering
4. remove the desktop markdown dependency only after ordinary messages also
   share the typed Rust-owned path
