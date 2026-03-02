## Spec

We are de-scoping all public-infrastructure integration testing from current CI and local default test contracts, and moving fully to deterministic/local fixture-backed integration coverage.

This is being done because current public/deployed tests are flaky and mostly duplicate behavior coverage already available (or achievable) through local Rust integration tests. The immediate goal is high-confidence, stable, fast local-first integration validation. Public-infra tests can be reintroduced later as deliberate canary lanes with explicit ownership and SLOs.

Core intent:
1. CI (pre-merge + nightly default lanes) should validate integration behavior using only local deterministic fixture-backed tests.
2. Any behavior currently covered only by public-infra tests must be migrated into local deterministic tests before public tests are deleted.
3. Public/deployed test ownership is removed for now (not merely skipped), along with wrapper/docs/guardrail references.
4. Selector-first architecture remains intact: Rust tests + `pikahut::testing` scenarios are the only integration contract.

Required outcome:
1. `integration_public` selectors are removed.
2. `crates/pikahut/src/testing/scenarios/public.rs` is removed unless a remaining local selector depends on it.
3. CI workflows and `just` recipes no longer invoke public-infra selectors.
4. Guardrails fail on reintroduction of public-infra selector paths into CI/default contracts.
5. A concise coverage parity doc proves which public assertions were already covered locally and which were newly migrated.
6. Docs are updated to local-first CI ownership and no longer claim nightly public coverage.

Coverage parity requirements (must be explicit):
1. Public UI ping/pong assertions are mapped to local UI E2E selectors (`ui_e2e_local_android`, `ui_e2e_local_ios`, desktop where applicable) with equivalent relay/bot behavior under local fixtures.
2. Deployed bot call-flow assertions (login, chat create, ping/pong, call active, frame flow, end call) are either:
- already covered by deterministic local call tests (`call_over_local_moq_relay`, `call_with_pikachat_daemon` and boundary selectors), or
- backfilled via new deterministic local selector(s).
3. Any truly external-only behavior is documented as intentionally deferred (not silently dropped).

Out of scope for this phase:
1. Reintroducing public canary lanes.
2. Shared fixture pooling optimization.
3. Large redesign of scenario architecture beyond removing public-infra paths.

## Plan

1. Add a coverage parity ledger before deleting public tests.
Acceptance criteria: a new doc in `docs/testing/` maps each `integration_public` assertion to existing local selector/test coverage; any uncovered assertion is listed as a required backfill item.

2. Backfill any uncovered assertions with deterministic local tests.
Acceptance criteria: missing behaviors identified in step 1 are implemented under local fixture-backed selectors/tests (prefer `integration_deterministic` + existing `pika_core` local e2e tests); new tests are deterministic and run without public network dependency.

3. Remove public integration selector target.
Acceptance criteria: `crates/pikahut/tests/integration_public.rs` is deleted; `cargo test -p pikahut --tests -- --list` no longer includes `integration_public` target selectors.

4. Remove public scenario module ownership in `pikahut::testing`.
Acceptance criteria: `crates/pikahut/src/testing/scenarios/public.rs` is removed (or reduced to local-only shared helpers if still needed); scenario module exports and callsites compile cleanly.

5. Remove public wrappers and recipes.
Acceptance criteria: `tools/ui-e2e-public` is deleted; `justfile` recipes for public/deployed flows (`e2e-public-relays`, public `android-ui-e2e`, public `ios-ui-e2e`, `e2e-deployed-bot`) are removed.

6. Repoint nightly/local-default lanes to local-only selectors.
Acceptance criteria: `just nightly-pika-e2e` runs only local deterministic/heavy local selectors; no public network requirements remain in that lane.

7. Update GitHub Actions workflow contracts to local-only integration lanes.
Acceptance criteria: `.github/workflows/pre-merge.yml` no longer runs public selector targets; lane descriptions/comments reflect local-only policy.

8. Update guardrails to enforce local-only CI integration contract.
Acceptance criteria: `crates/pikahut/tests/guardrails.rs` asserts:
- no `integration_public` selector invocations in CI lanes,
- no references to deleted public wrapper paths,
- required CI selector contract is local-only.

9. Remove now-unused dependencies introduced for public scenario orchestration.
Acceptance criteria: if unused after removal, `crates/pikahut/Cargo.toml` no longer depends on public-only crates (for example `pika_core`/`pika-relay-profiles` in pikahut crate), and `Cargo.lock` is updated.

10. Update canonical docs to reflect local-first ownership.
Acceptance criteria: at minimum update:
- `docs/testing/ci-selectors.md`
- `docs/testing/integration-matrix.md`
- `docs/testing/library-first-migration-checklist.md`
- `docs/testing/wrapper-deprecation-policy.md`
- `docs/testing/phase1-library-migration-closeout.md`
All references to default nightly/public selector ownership are removed or marked explicitly deferred.

11. Run formatting and test verification.
Acceptance criteria: run `cargo fmt`; run `cargo test -p pikahut --tests`; run `cargo test -p pikahut --test guardrails`; all pass.

12. Manual QA gate (user-run): validate local-first integration confidence.
Acceptance criteria: user runs representative local selectors (UI local android/ios where available, local call-path boundary selectors, deterministic OpenClaw deterministic suite) and confirms confidence is high without any public-infra lane.
