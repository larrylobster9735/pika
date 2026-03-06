# Native Semantic UI

Use this reference when the request is about making SwiftUI feel more native, especially for forms, profiles, settings, action rows, and screen-level hierarchy.

## Core Rule

Start with semantic SwiftUI containers and controls. Reach for raw `VStack` / `HStack` screen composition only after deciding that `Form`, `Section`, `List`, `LabeledContent`, `ToolbarItem`, `TextField`, `TextEditor`, `PasteButton`, `PhotosPicker`, or `ShareLink` cannot express the interaction cleanly.

Liquid Glass is important, but it comes after hierarchy. Use glass to reinforce structure, not to replace it.

## Container Selection

### Prefer `Form`
- Settings screens
- Profile editing screens
- Structured input flows with a few related fields
- Screens where grouped row spacing, keyboard behavior, and section semantics matter more than custom chrome

### Prefer `List`
- Read-only grouped content
- Mixed navigation rows, disclosure rows, and destructive rows
- Dynamic collections of rows that should inherit native swipe, focus, and accessibility behavior

### Prefer `LabeledContent`
- Read-only label/value presentation
- Account details, metadata, version info, and short profile facts
- Cases where a two-column display is desired without pretending the row is editable

### Prefer custom `ScrollView` + stacks
- Highly custom hero layouts
- Media-forward screens with strong visual composition
- Screens where system row styling fights the design and the semantics truly differ

Do not default to custom stacks for ordinary forms and settings screens.

## Edit vs Display

### Editable screens
- Use real controls: `TextField`, `SecureField`, `TextEditor`, `Toggle`, `Picker`
- Placeholder-first fields are often more native than a custom left-label layout
- Keep save actions and destructive actions visually distinct

### Display screens
- Use plain text, `LabeledContent`, grouped rows, or sectioned read-only surfaces
- Do not make read-only content look like an inactive edit form
- For viewer profiles, show name and bio as content, not as fake inputs

## Profile Patterns

### Self profile
- Use a `Form` or grouped `List` for editable fields
- Keep avatar/photo editing near the top
- Group shareable identity details together
- Keep private-key material visually separate from public/shareable material
- Settings belong in their own section with standard row semantics

### Other-user profile
- Use a display-first layout: avatar, name, bio, then actions
- Keep media/navigation rows separate from primary actions
- If actions are custom, ensure every button is still a semantic `Button`
- Reuse existing app actions; do not add speculative buttons

## Settings Rows

Prefer native row semantics:
- `NavigationLink` or button row for drill-in settings
- plain informational row for version/build details
- destructive button row for logout/delete/reset actions

Avoid making button rows accidentally look bolder than the surrounding settings unless that emphasis is intentional.

## Code Entry and Share Flows

- Prefer `PasteButton` when it fits the UX and platform target
- Keep manual entry as a secondary fallback if the product still supports it
- Use one terminology system consistently across the screen and related flows
- Share/import actions should preserve working accessibility IDs and tests when refactoring

## Action Rows

When the product needs a compact custom action row:
- use `Button` for every action
- keep one clear primary action
- demote secondary actions visually instead of making every button equally loud
- place navigation/media rows separately from primary communication actions

If the system row pattern works, prefer that over a bespoke action bar.

## Liquid Glass

Use Liquid Glass deliberately:
- after hierarchy, spacing, and semantics are already correct
- on clustered controls, floating actions, or grouped surfaces that benefit from depth
- with consistent shapes and tints across related elements

Do not use glass to hide weak grouping or compensate for missing semantic structure.

## Validation Loop

For UI-focused refactors:
1. Change one screen at a time.
2. Verify real app behavior in the simulator after each screen.
3. Check that no existing actions disappeared.
4. Check spacing and grouping with a squint test.
5. Check icon-only buttons for accessibility labels.
6. Check that no placeholder notes or debug copy shipped.

Simulator review is mandatory before calling a visual pass complete.
