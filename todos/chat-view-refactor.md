# Chat View Refactor

## Principle

Embrace native abstractions whenever possible.

- Never fight native layout, keyboard, safe-area, or scroll behavior.
- If a native abstraction does not support a behavior cleanly, stop and discuss it before adding a patch.
- No new bottom `safeAreaInset` ownership for the transcript.
- No manual keyboard frame math.
- No synthetic spacer rows or magic bottom padding unless they are owned by the same UIKit surface that owns the transcript.

## Current Baseline

Checkpoint:
- Commit `71548f5` established the accessory-host transcript architecture.
- Follow-up checkpoint should keep only the simulator voice-recorder fallback plus this plan doc.

What is working:
- Full-height `UICollectionView` transcript with accessory-backed composer.
- Keyboard-open behavior is mostly correct.
- No inverted list.

What is still wrong:
- Closed-keyboard state shows a white/bar-gap style bottom region.
- Dynamic accessory growth (staged media, likely voice recorder) is not integrated cleanly.
- Keyboard animation still has visual instability.
- The current bridge still has leftover fixed inset assumptions and update churn.

## Non-Negotiable Acceptance

- Full bleed remains.
- No bottom `safeAreaInset` for the transcript/composer relationship.
- Keyboard-open and keyboard-closed states both look native and consistent.
- Accessory growth pushes the visible transcript area correctly.
- No message bubble overlaps the composer.
- No white bar or disappearing background during keyboard animation.
- Input changes, bubble changes, and voice-recorder changes should not restart another safe-area fight.

## Rollback Triggers

Immediate thumbs-down / rewind if any of these appear:
- A bottom `safeAreaInset` is reintroduced for transcript ownership.
- Manual keyboard height observers or keyboard frame math show up.
- New magic bottom spacers or one-off padding constants are added to "fix" overlap.
- The transcript only works in one state (keyboard open or closed) but regresses in the other.
- The architecture becomes harder to explain than "UIKit owns transcript + accessory; SwiftUI owns screen chrome."

## Phases

### Phase 0: Checkpoint

Goal:
- Save the simulator voice-recorder fix and this plan in a standalone commit.

Acceptance:
- Clean rollback point exists before more layout work.

### Phase 1: Remove the Two Biggest Leftover Hacks

Goal:
- Remove the fixed bottom inset model from transcript layout.
- Stop calling `reloadInputViews()` on normal accessory updates.

Why:
- These are the two largest remaining sources of "fake" bottom geometry and animation churn.

Acceptance:
- No fixed transcript bottom reserve in `MessageCollectionLayout`.
- Accessory updates do not tear down/reload the keyboard stack on each state change.
- `just run-swift --sim` stays green.

Thumbs up:
- Closed-keyboard bottom looks more native.
- Keyboard animation becomes less flashy/flickery.

Thumbs down:
- White bar gets worse.
- Accessory disappears or detaches.

### Phase 2: Separate Content Changes From Height Changes

Goal:
- Stop treating every composer-state change as a viewport/layout event.

Why:
- Text edits, mention query changes, reply state, and staged media do not all mean the transcript should react the same way.

Acceptance:
- Only actual accessory height changes affect transcript geometry.
- Normal text-entry updates do not cause transcript repositioning.

Thumbs up:
- Typing feels inert with respect to transcript layout.
- Dynamic-height elements are the only things that move the viewport.

### Phase 3: Make the Accessory Height Contract Explicit

Goal:
- The host controller should own one clear bottom-geometry model:
  - keyboard boundary
  - accessory height
  - optional extra breathing room, if still needed

Why:
- Today this is implicit and partially split between host constraints and transcript inset logic.

Acceptance:
- Closed and open keyboard states use the same underlying geometry model.
- Staged media and voice recorder expand naturally without overlap.

### Phase 4: Shrink the Accessory Surface

Goal:
- Move non-essential or presentation-only UI out of the self-sizing accessory subtree if they do not need to live there.

Candidates to evaluate:
- mention picker
- some alert/presenter wiring
- any non-size-affecting state that causes accessory rebuilds

Acceptance:
- Accessory subtree mostly contains actual bottom chrome, not unrelated presenters.

### Phase 5: Hardening

Goal:
- Verify the architecture is stable under real chat operations.

Manual QA matrix:
- short chat, long chat, mid-history
- keyboard closed/open
- staged media on/off
- voice recorder shown/hidden
- reply draft shown/hidden
- typing and sending while pinned
- typing and sending while scrolled up

Acceptance:
- No regressions across the matrix.

## Architectural End State

The intended shape is simple:

- SwiftUI owns the screen, top chrome, sheets, fullscreen media, and product-level state.
- UIKit owns the transcript scroll view, accessory attachment, keyboard interaction, and bottom geometry.
- The contract between them is narrow and explicit.

If a future change to bubbles, composer, or recorder requires revisiting keyboard/safe-area ownership, that is a design discussion, not a patching exercise.
