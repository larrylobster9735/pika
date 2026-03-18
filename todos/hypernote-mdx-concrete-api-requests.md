---
summary: Concrete API-level requests for hypernote-mdx so downstream Rust crates can lower and render MDX without duplicating parser semantics
read_when:
  - sending concrete API requests to the hypernote-mdx team
  - moving generic Hypernote lowering into hypernote-protocol
  - deciding what parser semantics should stay in hypernote-mdx
---

# hypernote-mdx Concrete API Requests

## Context

Pika is moving toward this split:

1. `hypernote-mdx` owns parsing and parser-adjacent semantic accessors
2. `hypernote-protocol` owns a Rust-native typed document / lowering layer plus
   Pika-specific component and action semantics
3. Pika's SwiftUI, Kotlin, and Iced clients stay thin and switch over
   Rust-owned Hypernote concepts

This note is **not** about protocol-specific features like `SubmitButton`,
polls, Nostr actions, or form-state tags. Those belong above the parser.

This note is about what `hypernote-mdx` could export so downstream Rust crates
do not have to reverse-engineer parser semantics from raw AST internals.

## Important Design Preference

We do **not** want runtime discovery as the primary sync mechanism.

We want compile-time sync as much as possible:

1. explicit types
2. explicit helper methods
3. exhaustive enums / structs
4. downstream compile failures when parser-visible semantics change

So where "capabilities" came up earlier, the preferred shape here is **typed
surface area**, not a runtime `supported_features()` query.

Good examples:

1. `CodeBlockInfo`
2. `LinkInfo`
3. `ImageInfo`
4. `JsxAttributeValue`
5. a typed "tree" / semantic node export

Less preferred:

1. stringly feature flags
2. runtime inspection APIs as the only source of truth

## Why We Are Asking

Right now Pika has to duplicate parser semantics in
[rust/src/hypernote.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/rust/src/hypernote.rs#L11).

Concrete duplication points:

1. Code block extraction in
   [rust/src/hypernote.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/rust/src/hypernote.rs#L304)
2. Link/image extraction in
   [rust/src/hypernote.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/rust/src/hypernote.rs#L348)
3. Frontmatter extraction in
   [rust/src/hypernote.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/rust/src/hypernote.rs#L362)
4. Expression extraction in
   [rust/src/hypernote.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/rust/src/hypernote.rs#L385)
5. JSX attribute decoding / coercion in
   [rust/src/hypernote.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/rust/src/hypernote.rs#L407)
6. HTML entity and quoted-string decoding in
   [rust/src/hypernote.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/rust/src/hypernote.rs#L434)

That code works, but it means downstream crates must stay in sync with parser
implementation details like:

1. token ordering
2. `extra_data` layout
3. which token contains decoded or raw values
4. how JSX booleans and strings are represented
5. how fenced code blocks are sliced from source

That is exactly the kind of sync burden we want to push down into
`hypernote-mdx`.

## Concrete Requests

### 1. Public typed semantic accessors for core node kinds

Requested exports:

```rust
pub struct CodeBlockInfo<'a> {
    pub lang: Option<&'a str>,
    pub code: &'a str,
}

pub struct LinkInfo<'a> {
    pub label_children: &'a [NodeIndex],
    pub url: &'a str,
}

pub struct ImageInfo<'a> {
    pub alt_children: &'a [NodeIndex],
    pub url: &'a str,
}

pub struct ExpressionInfo<'a> {
    pub value: &'a str,
}

pub struct FrontmatterInfoView<'a> {
    pub format: FrontmatterFormat,
    pub value: &'a str,
}
```

Example methods:

```rust
impl Ast {
    pub fn code_block_info(&self, node: NodeIndex) -> CodeBlockInfo<'_>;
    pub fn link_info(&self, node: NodeIndex) -> LinkInfo<'_>;
    pub fn image_info(&self, node: NodeIndex) -> ImageInfo<'_>;
    pub fn expression_info(&self, node: NodeIndex) -> ExpressionInfo<'_>;
    pub fn frontmatter_view(&self, node: NodeIndex) -> FrontmatterInfoView<'_>;
}
```

Why this helps:

1. It removes direct downstream dependence on token slicing and `extra_data`
   layout.
2. It makes semantic changes surface at compile time when method signatures or
   structs change.
3. It reduces copy-paste logic across downstream crates.

### 2. Public typed JSX attribute values

Requested export:

```rust
pub enum JsxAttributeValue<'a> {
    String(&'a str),
    Number(f64),
    Boolean(bool),
    Expression(&'a str),
}

pub struct JsxAttributeView<'a> {
    pub name: &'a str,
    pub value: JsxAttributeValue<'a>,
}
```

Example methods:

```rust
impl Ast {
    pub fn jsx_attribute_views(&self, node: NodeIndex) -> Vec<JsxAttributeView<'_>>;
}
```

Behavior we want:

1. Strings come back already unquoted and entity-decoded
2. Numbers come back as typed numeric values
3. Boolean attributes are explicit
4. Expressions are explicit

Why this helps:

1. Pika currently has to duplicate attribute coercion and decoding logic in
   [rust/src/hypernote.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/rust/src/hypernote.rs#L407).
2. This is parser-adjacent logic, not app logic.

### 3. Public plain-text extraction helpers

Requested exports:

```rust
impl Ast {
    pub fn plain_text(&self, node: NodeIndex) -> String;
    pub fn plain_text_children(&self, children: &[NodeIndex]) -> String;
}
```

Behavior we want:

1. `text` contributes literal text
2. `strong` / `emphasis` recurse
3. `code_inline` contributes its visible text
4. `link` contributes label text if present, otherwise URL
5. `hard_break` contributes `\n`

Why this helps:

1. Every renderer ends up rebuilding some version of this helper.
2. It is not app-specific.

### 4. A typed semantic tree export in Rust, not just JSON

Requested export:

```rust
pub struct TreeNode {
    pub node_type: TreeNodeType,
    pub children: Vec<TreeNode>,
    pub value: Option<String>,
    pub level: Option<u8>,
    pub url: Option<String>,
    pub lang: Option<String>,
    pub name: Option<String>,
    pub checked: Option<bool>,
    pub attributes: Vec<TreeAttribute>,
}
```

This does **not** need to become the one true app boundary for every client.
But a typed Rust tree export would provide a stable parser-owned semantic layer
for downstream lowering.

Why this helps:

1. `serialize_tree()` already implies semantic decisions beyond the raw AST.
2. Downstream Rust code should be able to reuse those semantics without going
   through JSON.

### 5. A small public "semantic lowering helpers" module

If a full typed semantic tree feels too high-level for the parser crate, a
smaller alternative would still help a lot:

```rust
pub mod semantic {
    pub fn decode_jsx_string(raw: &str) -> String;
    pub fn decode_html_entities(raw: &str) -> String;
    pub fn extract_code_block(ast: &Ast, node: NodeIndex) -> CodeBlockInfo<'_>;
    pub fn extract_link(ast: &Ast, node: NodeIndex) -> LinkInfo<'_>;
}
```

This is less ideal than typed node accessors on `Ast`, but still much better
than every downstream crate copying internal parser logic.

### 6. Prefer additive, typed parser surface over runtime feature reporting

Instead of a runtime `capabilities()` API, we would prefer:

1. new typed nodes / helpers when syntax support is added
2. compile-time discoverability from public enums and methods
3. tests that prove those helpers stay aligned with the parser

If there is a feature-matrix need for docs or CI, a const/static table is fine,
but we would not want downstream runtime behavior to depend on probing it.

## Suggested Upstream Test Shapes

### 1. Accessor parity tests

For every semantic helper, test it directly from parsed source.

Examples:

1. fenced code block fixture -> `code_block_info()` returns expected `lang` and
   `code`
2. `[label](url)` fixture -> `link_info()` returns `label_children` and `url`
3. `![alt](url)` fixture -> `image_info()` returns `alt_children` and `url`
4. JSX fixture -> `jsx_attribute_views()` returns decoded typed attrs

### 2. Tree-builder parity tests

If `serialize_tree()` remains supported, semantic helpers and typed tree export
should agree with it.

Examples:

1. `code_block_info()` matches serialized `lang` / `value`
2. `link_info()` matches serialized `children` / `url`
3. `jsx_attribute_views()` matches serialized attribute `value_type` / `value`

### 3. Snapshot tests on representative fixtures

Fixtures should cover:

1. plain markdown
2. markdown mixed with JSX
3. task lists
4. tables
5. nested emphasis / links / code
6. frontmatter
7. expressions

Suggested assertion style:

1. typed tree snapshot
2. serialized JSON snapshot
3. helper-level assertions on important fields

### 4. Negative / edge-case tests

Examples:

1. empty link label
2. boolean JSX attrs with and without explicit `=true`
3. malformed quoted strings
4. code fences with missing language
5. nested inline content around links and emphasis

These tests matter because downstream lowering code is where sync bugs usually
show up first.

## What Should Stay Out Of hypernote-mdx

These asks are intentionally **not** requesting:

1. Pika component registries
2. Nostr / Hypernote action semantics
3. `SubmitButton` action extraction
4. form-state tags
5. FFI-oriented document shapes
6. client rendering concerns

Those belong in `hypernote-protocol` and Pika.

## Concrete Near-Term Value For Pika

If `hypernote-mdx` provided the semantic helpers above, we could remove a large
chunk of parser-semantic duplication from
[rust/src/hypernote.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/rust/src/hypernote.rs#L101)
and keep the downstream layer focused on:

1. defining a generic typed document in `hypernote-protocol`
2. defining Pika-specific components and actions in `hypernote-protocol`
3. keeping client renderers like
   [crates/pika-desktop/src/views/hypernote.rs](/Users/futurepaul/dev/sec/other-peoples-code/pika/crates/pika-desktop/src/views/hypernote.rs#L49)
   thin and semantic

That is the main goal: thin clients, pure Rust Hypernote logic, and good
compile-time sync between parser support and downstream lowering.
