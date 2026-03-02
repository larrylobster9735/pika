## Spec

Close gaps between the documented integration matrix and what actually runs in CI. The selector migration is complete, but several selectors that should be running in pre-merge or nightly are not wired into the justfile recipes or CI workflows.

Context: the integration matrix (`docs/testing/integration-matrix.md`) lists `desktop-e2e-local` as a pre-merge-pikachat selector and `cli-smoke-media` as nightly, but neither actually runs in any CI lane today. The nightly Android lane only runs `NostrConnectIntentTest` when the full `PikaUiTest` is available at no extra cost. The primal-ios-interop nightly is gated behind a variable that may never have been enabled.

## Plan

1. Add `desktop-e2e-local` to `pre-merge-pikachat`.
Acceptance criteria: `justfile` `pre-merge-pikachat` recipe includes `cargo test -p pikahut --test integration_deterministic ui_e2e_local_desktop -- --ignored --nocapture`. This is deterministic, runs on Linux, requires no special capabilities.

2. Add `cli-smoke-media` to `nightly-pika-e2e`.
Acceptance criteria: `justfile` `nightly-pika-e2e` recipe includes `cargo test -p pikahut --test integration_deterministic cli_smoke_media_local -- --ignored --nocapture`. Nightly already hits public infrastructure; media upload/download via Blossom should be covered there.

3. Run full `PikaUiTest` in `nightly-pika-ui-android`, not just `NostrConnectIntentTest`.
Acceptance criteria: the `nightly-pika-ui-android` CI job drops the `PIKA_ANDROID_E2E_TEST_CLASS` filter so the full instrumentation suite runs. The emulator is already booted and the app is already installed.

4. Enable or remove the `nightly-primal-ios-interop` gate.
Acceptance criteria: either set `PIKA_NIGHTLY_PRIMAL_INTEROP=1` in the repo variables so it actually runs, or remove the condition if it was never intentionally disabled. If it is intentionally off, add a comment in the workflow explaining why.

5. Add iOS Swift unit tests to a macOS nightly lane.
Acceptance criteria: `just ios-ui-test` (which runs `AppManagerTests` + `KeychainNsecStoreTests` and skips the deployed-bot E2E) runs in the `nightly-primal-ios-interop` job or a new macOS nightly job. These are fast (~2s), deterministic, and test the real iOS glue layer.

6. Update docs to match reality.
Acceptance criteria: `docs/testing/ci-selectors.md` and `docs/testing/integration-matrix.md` reflect the actual lane assignments after steps 1-5. No rows claim a test runs in a lane where it doesn't.
