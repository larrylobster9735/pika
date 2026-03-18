---
summary: Multi-stage plan to remove hypernote JSON boundaries, add typed Hypernote rendering to desktop, unify chat rendering through Rust-driven hypernote parsing, and benchmark client performance
read_when:
  - working on hypernote rendering or parser boundaries
  - planning iOS/Android markdown stack simplification
  - planning desktop/Iced markdown stack simplification
  - evaluating Rust-to-native UI data boundaries
---

# Hypernote Unified Rendering Plan

## Problem Statement

Pika currently treats hypernotes as a special path:

1. Rust parses MDX with `hypernote-mdx`.
2. Rust serializes the parsed tree to `ast_json`.
3. Swift and Kotlin parse that JSON again into native AST structs.
4. Desktop now renders ordinary chat markdown through a separate
   `pulldown-cmark` path added in PR `#644` on March 15, 2026.
5. Regular chat messages still use client-specific markdown libraries instead
   of the Rust hypernote parser.

That means we currently pay for:

1. Rust parse + JSON serialize
2. Native JSON decode on iOS and Android
3. Separate client markdown stacks for normal chat messages:
   `MarkdownUI`, `compose-markdown`, and `pulldown-cmark`
4. No real desktop Hypernote renderer or action path yet
5. Duplicated rendering/parsing logic across Rust, Swift, Kotlin, and Iced

The target direction is simpler:

1. Rust owns the parse step for both hypernotes and regular message content.
2. Pika owns a typed render-oriented content model rather than exposing parser
   internals or AST JSON.
3. UniFFI carries typed data across the mobile boundary, and desktop consumes
   the same typed model directly in Rust.
4. Iced, iOS, and Android render from that typed data instead of reparsing or
   using client-local markdown parsing.
5. Client markdown dependencies disappear once hypernote rendering is broad
   enough and fast enough.

This plan is intentionally staged. The early goal is not "replace everything at
once." The early goal is to remove obviously unnecessary work and make the
final migration straightforward.

## Acceptance Criteria

This project is done when all of the following are true:

1. Existing hypernotes no longer cross the Rust/mobile boundary as AST JSON.
2. Existing non-hypernote chat messages also use Rust-driven hypernote parsing
   or an equivalent typed Rust-owned content model.
3. Desktop renders Hypernotes from typed Rust-owned data rather than skipping
   them or depending on AST JSON in the UI layer.
4. iOS no longer depends on `MarkdownUI` for chat message rendering.
5. Android no longer depends on `compose-markdown` for chat message rendering.
6. Desktop no longer depends on `pulldown-cmark` for chat message rendering.
7. We have a repeatable way to benchmark parsing/rendering work inside real
   client builds, not just in Rust.
8. Runtime behavior and visual coverage are good enough that removing client
   markdown libraries is a net simplification rather than a regression.

## Constraints

Musts:

1. Prefer typed UniFFI records/enums over JSON strings when the data shape is
   owned by Pika.
2. Keep the migration incremental and landable in small slices.
3. Preserve current hypernote behavior while changing the transport boundary.
4. Add measurement before claiming mobile performance wins.
5. Prefer Rust as the source of truth for parse/segment logic.
6. Let desktop Hypernote support land before ordinary desktop markdown
   unification if that keeps the rollout smaller and safer.

Must-nots:

1. Do not block the whole project on designing the perfect final AST shape.
2. Do not remove native markdown dependencies before feature coverage and
   performance are acceptable.
3. Do not add a second long-term serialization format just to replace JSON.
4. Do not assume Rust microbenchmarks alone answer mobile performance.

Preferences:

1. Prefer a render-oriented typed model over exporting the raw parser-internal
   AST unchanged.
2. Prefer cacheable immutable message-derived state because Pika messages are
   not edited after send.
3. Prefer one shared content pipeline for hypernotes and normal messages when
   practical.

Escalations:

1. If UniFFI shape limits make the chosen typed tree too awkward, re-evaluate
   the boundary before implementing deep native changes.
2. If full markdown parity is materially harder than expected, keep hypernotes
   and regular markdown on separate renderers temporarily, but keep Rust as the
   parse/segment owner.
3. If the unified renderer is slower on-device, do not remove native markdown
   deps until the gap is understood.
4. If `hypernote-mdx` parity gaps block ordinary desktop markdown migration,
   keep `pulldown-cmark` temporarily while typed Hypernotes still land on
   desktop.

## Current Reality

Today:

1. `ChatMessage` and `HypernoteData` already cross FFI as UniFFI records.
2. `HypernoteData` still contains `ast_json: String` and `default_state:
   Option<String>`.
3. iOS and Android both decode `ast_json` locally and render from native AST
   structs.
4. Desktop ordinary chat messages now render through a separate
   `pulldown-cmark` Iced renderer.
5. Desktop still does not branch on `message.hypernote` and has no Hypernote
   action plumbing yet.
6. Normal chat messages still render through client-local markdown libraries.
7. Android already memoizes decoded hypernote AST per message; iOS currently
   reparses in the renderer path.

That means there is an immediate local improvement available even before the
full unification project: stop using JSON for existing hypernotes and make
desktop the first consumer of the typed Hypernote model.

## Proposed Decomposition

### Stage 1 — Remove JSON for Existing Hypernotes

Goal:
Make current hypernotes cross FFI as typed data, not AST JSON strings.

Concrete Stage 1 shape:

1. Add a Pika-owned typed hypernote render model to `HypernoteData`.
2. Keep that model close to the current renderer inputs (`type`, `value`,
   `level`, `url`, `lang`, `name`, `attributes`) so mobile code can switch
   without redesigning rendering behavior.
3. Represent the tree as a flat arena (`nodes` + `root_node_ids` + `child_ids`)
   rather than recursive UniFFI records. This matches `hypernote_mdx`'s
   internal arena layout and avoids recursive-type friction at the FFI
   boundary.
4. Normalize the current `state` tag in Rust into a typed form-state map
   instead of shipping a JSON string to mobile.
5. Compute `declared_actions` from the typed document, not from serialized AST
   JSON.

Stage 1 scope lock:

1. Only cover the markdown and JSX node shapes already rendered today on both
   mobile platforms.
2. Current markdown node surface:
   `heading`, `paragraph`, `strong`, `emphasis`, `code_inline`, `code_block`,
   `link`, `image`, `list_unordered`, `list_ordered`, `list_item`,
   `blockquote`, `hr`, `hard_break`, `text`.
3. Current JSX component surface:
   `Card`, `VStack`, `HStack`, `Heading`, `Body`, `Caption`, `TextInput`,
   `SubmitButton`, `Details`, `Summary`, `ChecklistItem`.
4. Parser nodes outside that surface can map to `Unsupported` for Stage 1.
   Tables, frontmatter, and MDX expression fidelity can stay out of scope until
   later stages because the current mobile renderers do not meaningfully handle
   them either.

Proposed UniFFI types:

```rust
#[derive(uniffi::Record, Clone, Debug)]
pub struct HypernoteDocument {
    pub root_node_ids: Vec<u32>,
    pub nodes: Vec<HypernoteNode>,
}

#[derive(uniffi::Record, Clone, Debug)]
pub struct HypernoteNode {
    pub id: u32,
    pub node_type: HypernoteNodeType,
    pub child_ids: Vec<u32>,
    pub value: Option<String>,
    pub level: Option<u8>,
    pub url: Option<String>,
    pub lang: Option<String>,
    pub name: Option<String>,
    pub raw_type_name: Option<String>,
    pub attributes: Vec<HypernoteAttribute>,
}

#[derive(uniffi::Enum, Clone, Debug, PartialEq, Eq)]
pub enum HypernoteNodeType {
    Heading,
    Paragraph,
    Strong,
    Emphasis,
    CodeInline,
    CodeBlock,
    Link,
    Image,
    ListUnordered,
    ListOrdered,
    ListItem,
    Blockquote,
    Hr,
    HardBreak,
    Text,
    MdxJsxElement,
    MdxJsxSelfClosing,
    Unsupported,
}

#[derive(uniffi::Record, Clone, Debug)]
pub struct HypernoteAttribute {
    pub name: String,
    pub value_type: HypernoteAttributeValueType,
    pub value: Option<String>,
}

#[derive(uniffi::Enum, Clone, Debug, PartialEq, Eq)]
pub enum HypernoteAttributeValueType {
    String,
    Number,
    Boolean,
    Expression,
}

#[derive(uniffi::Record, Clone, Debug)]
pub struct HypernoteData {
    // Transitional Stage 1 shape:
    // keep legacy ast_json/default_state until both mobile renderers switch.
    pub document: HypernoteDocument,
    pub default_form_state: HashMap<String, String>,
    pub declared_actions: Vec<String>,
    pub title: Option<String>,
    pub my_response: Option<String>,
    pub response_tallies: Vec<HypernoteResponseTally>,
    pub responders: Vec<HypernoteResponder>,
}
```

Notes on the proposed shape:

1. This is intentionally not the raw `hypernote_mdx::Ast`. The parser AST
   includes token tables, spans, `extra_data`, and other parser-oriented
   details that mobile renderers do not need.
2. The model is also intentionally not a deeply UI-specific "render segment"
   API yet. Stage 1 should only replace JSON transport, not prematurely solve
   Stage 2.
3. `default_form_state` should be normalized in Rust so `{"foo":true}` and
   `{"gap":8}` arrive as plain string values mobile can immediately seed into
   its existing mutable form-state map.

Affected files for Stage 1:

Rust:

1. [rust/src/state.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/rust/src/state.rs)
   Add `HypernoteDocument`, `HypernoteNode`, `HypernoteAttribute`, the related
   enums, and the new `HypernoteData` fields.
2. New file:
   [rust/src/hypernote.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/rust/src/hypernote.rs)
   Convert `hypernote_mdx::Ast` into `HypernoteDocument`, normalize default
   form state, and expose typed traversal helpers.
3. [rust/src/lib.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/rust/src/lib.rs)
   Wire in the new module.
4. [rust/src/core/storage.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/rust/src/core/storage.rs)
   Build `HypernoteDocument` when constructing `ChatMessage`, replace
   JSON-based declared-action extraction, and update tests that currently
   fabricate `HypernoteData { ast_json, .. }`.
5. [crates/hypernote-protocol/src/lib.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/crates/hypernote-protocol/src/lib.rs)
   Replace `extract_submit_actions_from_ast_json` with a typed equivalent such
   as `extract_submit_actions_from_document`.
6. Generated bindings:
   [ios/Bindings/pika_core.swift](/Users/futurepaul/dev/sec/other-peoples-code/pika/ios/Bindings/pika_core.swift)
   and
   [android/app/src/main/java/com/pika/app/rust/pika_core.kt](/Users/futurepaul/dev/sec/other-peoples-code/pika/android/app/src/main/java/com/pika/app/rust/pika_core.kt)
   Regenerated from the Rust type changes.

iOS:

1. [ios/Sources/Views/HypernoteRenderer.swift](/Users/futurepaul/dev/sec/other-peoples-code/pika/ios/Sources/Views/HypernoteRenderer.swift)
   Remove the local `Decodable` AST mirror and render from the typed document.
2. [ios/Sources/Views/MessageBubbleViews.swift](/Users/futurepaul/dev/sec/other-peoples-code/pika/ios/Sources/Views/MessageBubbleViews.swift)
   Switch the renderer call site from `astJson/defaultState` to the typed
   hypernote fields.
3. Optional small helper if the renderer gets too dense:
   [ios/Sources/Views/HypernoteRenderer.swift](/Users/futurepaul/dev/sec/other-peoples-code/pika/ios/Sources/Views/HypernoteRenderer.swift)
   can grow a tiny local document index (`[UInt32: HypernoteNode]`) rather than
   re-scanning `document.nodes` on every recursive render call.

Android:

1. [android/app/src/main/java/com/pika/app/ui/screens/HypernoteRenderer.kt](/Users/futurepaul/dev/sec/other-peoples-code/pika/android/app/src/main/java/com/pika/app/ui/screens/HypernoteRenderer.kt)
   Remove `JSONObject` parsing and render from the typed document.
2. [android/app/src/main/java/com/pika/app/ui/screens/ChatScreen.kt](/Users/futurepaul/dev/sec/other-peoples-code/pika/android/app/src/main/java/com/pika/app/ui/screens/ChatScreen.kt)
   Likely unchanged or only lightly touched, because it already passes the full
   `HypernoteData` record into the renderer.

Desktop:

1. New file:
   [crates/pika-desktop/src/views/hypernote.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/crates/pika-desktop/src/views/hypernote.rs)
   Render typed Hypernote documents in Iced.
2. [crates/pika-desktop/src/views/message_bubble.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/crates/pika-desktop/src/views/message_bubble.rs)
   Branch on `msg.hypernote` instead of only rendering markdown segments.
3. [crates/pika-desktop/src/views/conversation.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/crates/pika-desktop/src/views/conversation.rs)
   Add Hypernote UI messages/events for submit actions and local form state.
4. [crates/pika-desktop/src/screen/home.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/crates/pika-desktop/src/screen/home.rs)
   Dispatch desktop Hypernote events into `AppAction::HypernoteAction`.
5. [crates/pika-desktop/src/views/mod.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/crates/pika-desktop/src/views/mod.rs)
   Wire in the new Iced Hypernote module.

Smallest safe migration slices:

1. Slice A: Rust dual-write foundation.
   Add `document` and `default_form_state` to `HypernoteData`, keep
   `ast_json` and string `default_state` temporarily, build both in
   `build_chat_message`, and switch declared-action extraction to the typed
   document immediately. Land this with no mobile behavior change.
   Files:
   [rust/src/state.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/rust/src/state.rs),
   [rust/src/hypernote.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/rust/src/hypernote.rs),
   [rust/src/core/storage.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/rust/src/core/storage.rs),
   [crates/hypernote-protocol/src/lib.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/crates/hypernote-protocol/src/lib.rs),
   plus regenerated bindings.
2. Slice B: Desktop read-only renderer switch.
   Add an Iced Hypernote renderer over `hypernote.document` and branch on
   `msg.hypernote` in the desktop message bubble. Keep ordinary desktop
   markdown on `pulldown-cmark` for now.
   Files:
   [crates/pika-desktop/src/views/hypernote.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/crates/pika-desktop/src/views/hypernote.rs),
   [crates/pika-desktop/src/views/message_bubble.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/crates/pika-desktop/src/views/message_bubble.rs),
   [crates/pika-desktop/src/views/mod.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/crates/pika-desktop/src/views/mod.rs)
3. Slice C: Desktop action plumbing.
   Add conversation-level Hypernote events and dispatch them from desktop into
   `AppAction::HypernoteAction`. Manage local form state for `TextInput` and
   `ChecklistItem`.
   Files:
   [crates/pika-desktop/src/views/conversation.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/crates/pika-desktop/src/views/conversation.rs),
   [crates/pika-desktop/src/screen/home.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/crates/pika-desktop/src/screen/home.rs),
   [crates/pika-desktop/src/views/hypernote.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/crates/pika-desktop/src/views/hypernote.rs)
4. Slice D: Android renderer switch.
   Replace `parseHypernoteAst`, `parseNode`, `parseAttributes`, and
   `parseDefaultState` with helpers over `hypernote.document` and
   `hypernote.defaultFormState`. Keep current UI behavior and current action
   callback contract unchanged.
   File:
   [android/app/src/main/java/com/pika/app/ui/screens/HypernoteRenderer.kt](/Users/futurepaul/dev/sec/other-peoples-code/pika/android/app/src/main/java/com/pika/app/ui/screens/HypernoteRenderer.kt)
5. Slice E: iOS renderer switch.
   Remove `HypernoteAstNode`, `HypernoteAstAttribute`, `parseAst()`, and the
   `JSONSerialization` default-state decode. Render from `hypernote.document`
   and seed `interactionState` from `hypernote.defaultFormState`.
   Files:
   [ios/Sources/Views/HypernoteRenderer.swift](/Users/futurepaul/dev/sec/other-peoples-code/pika/ios/Sources/Views/HypernoteRenderer.swift)
   and
   [ios/Sources/Views/MessageBubbleViews.swift](/Users/futurepaul/dev/sec/other-peoples-code/pika/ios/Sources/Views/MessageBubbleViews.swift)
6. Slice F: Remove legacy JSON fields.
   Delete `ast_json` and string `default_state` from `HypernoteData`, remove
   dead JSON helpers and tests, regenerate bindings one last time, and confirm
   there are no remaining `astJson`/`defaultState` call sites outside generated
   files.

Verification for Stage 1:

1. Rust unit coverage:
   add tests for `Ast -> HypernoteDocument` conversion, default-state
   normalization, and typed submit-action extraction.
2. Existing storage tests in
   [rust/src/core/storage.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/rust/src/core/storage.rs)
   should switch from asserting `!hn.ast_json.is_empty()` to asserting the
   document is populated and actions are preserved.
3. Android UI coverage:
   [android/app/src/androidTest/java/com/pika/app/PikaE2eUiTest.kt](/Users/futurepaul/dev/sec/other-peoples-code/pika/android/app/src/androidTest/java/com/pika/app/PikaE2eUiTest.kt)
   already exercises `Details` and code-block behavior; keep that green through
   the renderer swap.
4. iOS UI coverage:
   [ios/UITests/PikaUITests.swift](/Users/futurepaul/dev/sec/other-peoples-code/pika/ios/UITests/PikaUITests.swift)
   already exercises the same `Details` and code-block path; keep that green
   through the renderer swap.
5. Desktop behavior coverage:
   add Iced view/event tests or focused state tests that verify Hypernote
   submit actions flow from the renderer through
   `views::conversation::Event` into `AppAction::HypernoteAction`.
6. Final grep-level completion check for Stage 1:
   no non-generated source file should reference `ast_json`, `astJson`,
   `JSONObject(astJson)`, `JSONDecoder().decode(HypernoteAstNode`,
   `JSONSerialization.jsonObject(with: ... defaultState ...)`, or desktop-side
   Hypernote JSON parsing helpers.

Acceptance for Stage 1:

1. No JSON AST field remains in `HypernoteData` after Slice F.
2. No iOS/Android/desktop Hypernote AST JSON decode remains.
3. Hypernote action extraction no longer depends on AST JSON.
4. Existing hypernotes still render and submit actions correctly on iOS,
   Android, and desktop.

### Stage 2 — Route Normal Chat Messages Through Hypernote Parsing With Runtime Caching

Goal:
Use the same Rust-owned parse/render pipeline for ordinary chat messages, not
just hypernotes.

Expected outcome:

1. Most or all message content is parsed in Rust.
2. Desktop, iOS, and Android render typed content from Rust for normal messages
   too.
3. Runtime caching avoids reparsing unchanged messages.
4. Hypernote becomes the main message rendering path rather than a niche path.

Why this is plausible:

1. Pika messages are effectively immutable after send.
2. There is no message editing workflow to invalidate caches frequently.
3. Message-derived render state is therefore a straightforward caching target.

Preferred cache shape:

1. Cache by stable message identity plus content hash/version if needed.
2. Treat parsed/render-ready content as immutable derived state.
3. Keep cache invalidation boring and conservative.

Questions to leave open for implementation:

1. Whether caching should live in Rust, native runtime state, or both.
2. Whether the shared output should be "full typed AST" or "render-ready
   content segments."
3. Whether normal markdown should share the exact hypernote renderer or a close
   sibling renderer over the same typed model.
4. Whether desktop should keep a temporary `pulldown-cmark` bridge longer than
   mobile if `hypernote-mdx` parity gaps remain for ordinary chat markdown.

Desktop sequencing note:

1. Desktop Hypernote rendering should land before this stage.
2. This stage may temporarily leave ordinary desktop markdown on
   `pulldown-cmark` if parser parity gaps remain for strikethrough, underscore
   emphasis, or rich inline link labels.

Acceptance for Stage 2:

1. Regular markdown messages no longer depend on client-local markdown parsing,
   except for any explicitly temporary desktop bridge called out during the
   migration.
2. Repeated rendering of the same messages does not repeatedly reparse content.
3. The new shared path handles the message shapes we actually send in chat.

### Stage 3 — Remove Native Markdown Dependencies

Goal:
Delete the client markdown libraries once the shared typed path is proven good
enough.

Expected outcome:

1. iOS chat rendering no longer depends on `MarkdownUI`.
2. Android chat rendering no longer depends on `compose-markdown`.
3. Desktop chat rendering no longer depends on `pulldown-cmark`.
4. Message rendering behavior is owned by Pika rather than split across
   platform-specific markdown stacks.

Important rule:

1. Dependency removal happens after coverage and performance confidence, not
   before.

Acceptance for Stage 3:

1. Markdown library deps are removed from client app targets.
2. Message rendering still covers the supported chat feature set.
3. There is no hidden fallback path quietly preserving the old libraries.

### Stage 4 — Validate Correctness and UX

Goal:
Prove the new path works well enough to keep.

Expected outcome:

1. Hypernotes still work on iOS, Android, and desktop.
2. Regular chat still looks right on iOS, Android, and desktop.
3. Copy, links, lists, code blocks, emphasis, images, and interactive hypernote
   controls still behave correctly.
4. No obvious regressions appear in long chat threads, scrolling, or repeated
   re-render cases.

What "works well" should mean here:

1. Visual parity on common message shapes.
2. Functional parity on links, code blocks, lists, and interactive controls.
3. No obvious frame drops or repeated parse churn while scrolling.
4. No spike in crashiness from recursive or malformed content.

Acceptance for Stage 4:

1. The shared renderer is stable in normal chat usage.
2. Manual QA and automated coverage are good enough to trust the dependency
   removal.

### Stage 5 — Add Native Benchmarking

Goal:
Benchmark the real client path so we can compare:

1. old native markdown path
2. JSON-based hypernote path
3. typed hypernote path
4. cached unified path

This stage is deliberately late in the plan because the right benchmark surface
depends on the final migrated path.

What matters most:

1. Benchmark representative chat-shaped inputs.
2. Measure cold parse and warm render separately when possible.
3. Measure repeated display of identical immutable messages.
4. Capture enough timing signal to compare alternatives honestly on real
   devices or realistic simulators/emulators.

What does not need to be solved right now:

1. The exact benchmark harness API.
2. The final reporting format.
3. Whether the first version uses XCTest/Android instrumentation, custom debug
   screens, or a small internal benchmark mode.

Acceptance for Stage 5:

1. iOS can measure the migrated path in a repeatable way.
2. Android can measure the migrated path in a repeatable way.
3. Desktop can measure or profile the migrated path in a repeatable way.
4. We can compare before/after numbers for representative chat workloads.

## Suggested Implementation Order

1. Change the hypernote boundary to typed data first.
2. Remove Rust-side AST-JSON consumers.
3. Make desktop render typed hypernotes directly.
4. Add desktop Hypernote action plumbing.
5. Make iOS and Android render typed hypernotes directly.
6. Add immutable runtime caching for parsed/render-ready message content.
7. Expand the Rust-owned content path to normal messages.
8. Delete `MarkdownUI`, `compose-markdown`, and `pulldown-cmark` only after
   parity confidence.
9. Add client benchmarking once the migrated path is stable enough to measure.

## Evaluation Design

What good tests look like in this project:

1. Rust tests that verify the typed hypernote/content model produced from real
   chat-shaped inputs.
2. iOS, Android, and desktop tests that exercise rendering of representative
   message fixtures, especially code blocks, lists, links, images, and interactive
   hypernotes.
3. Regression fixtures for malformed or surprising content so the typed path
   fails safely.
4. Repeated-render tests or profiling runs that confirm caching actually avoids
   duplicate parse work.

What good performance evaluation looks like:

1. Separate parse cost from display cost where possible.
2. Benchmark repeated render of identical messages because that is common in
   scrolling/chat re-entry scenarios.
3. Compare old and new paths side by side during the migration rather than only
   after the old path is deleted.
4. Prefer representative message corpora over tiny synthetic markdown samples.

## Open Questions

1. What exact typed model should replace `ast_json`?
2. Should `default_state` become `Map<String, String>` or a more specific typed
   form-state model?
3. Should Rust expose a raw node tree, a flattened node list, or a render-ready
   content model?
4. Where should the primary runtime cache live?
5. How much markdown coverage is required before deleting native markdown deps?
6. What is the smallest useful native benchmarking harness we can trust?
7. Which `hypernote-mdx` parity gaps, if any, should be closed before removing
   `pulldown-cmark` from desktop ordinary-message rendering?

## Non-Goals for the First Slice

1. Do not solve every final renderer design question before starting.
2. Do not merge hypernotes and normal markdown into one perfect architecture in
   the first step.
3. Do not remove MarkdownUI or compose-markdown in the same slice as the FFI
   boundary change.
4. Do not remove `pulldown-cmark` in the same slice as initial typed Hypernote
   work on desktop.
5. Do not overfit the boundary to today's parser internals if a slightly more
   stable Pika-owned model is clearer.

## Recommended First Deliverable

The first deliverable should be narrow and high-signal:

1. Replace hypernote AST JSON with a typed UniFFI boundary.
2. Replace JSON-based hypernote action extraction with typed traversal.
3. Replace `default_state` JSON with typed state if that falls out naturally.
4. Add typed Hypernote rendering and action plumbing to desktop Iced.
5. Keep ordinary chat markdown renderers unchanged for that first slice.

If that lands cleanly, the rest of the plan becomes much lower risk.
