## Spec

This is the canonical shared-fixtures todo for `pikahut`.

We need one active plan that reflects the repo as it exists today, without stale overlap across older shared-fixture todos/docs.

Current implementation state (verified in code):
1. Fixtures run in strict per-test mode today (`FixtureHandle::start` starts fixture components per test).
2. `TenantNamespace` helper exists with both `relay_namespace()` and `moq_namespace()` in `crates/pikahut/src/testing/tenant.rs`.
3. OpenClaw and interop scenarios attach tenant namespace metadata in scenario outputs.
4. Guardrail `shared_capable_scenarios_use_tenant_namespace_helpers()` enforces helper usage in those scenarios.

Not implemented yet:
1. No shared pool module (`pool.rs`), no `SharedInfra`, no `tokio::sync::OnceCell` infra singleton.
2. No `FixtureLifecycle` (`PerTest` / `SharedPerSuite`) on `FixtureSpec`.
3. No `DatabaseHandle` with per-test `CREATE DATABASE` / `DROP DATABASE`.
4. No runtime toggles for shared mode (`PIKAHUT_SHARED_FIXTURES`, `PIKAHUT_STRICT_MODE`).
5. No strict-vs-shared parity and isolation regression suite (relay/moq/postgres).
6. No deterministic CI lane wiring for shared mode.

Required outcome:
1. Shared-fixture architecture is implemented and isolated (relay/postgres/moq sharing only; bot/server remain per-test).
2. Strict mode remains available and testable as fallback.
3. Shared-mode promotion is evidence-based (parity + isolation + reliability + runtime delta), not assumed.
4. Shared defaults are never enabled for lanes/profiles lacking evidence; strict remains default there.
5. Shared pool failure policy is explicit: transient init failures recover; irrecoverable states fail fast.
6. Docs/guardrails/lane wiring stay consistent with real behavior.

Non-goals for this phase:
1. Sharing bot/server actors.
2. Broad shared defaults without evidence.
3. Rewriting unrelated integration migration scope.

## Plan

1. Canonicalize shared-fixture plan ownership in this file.
Acceptance criteria: this file is the only active shared-fixture implementation todo; overlapping shared-fixture todo files are deleted or superseded.

2. Remove relay namespace helper from tenant API.
Acceptance criteria: `TenantNamespace::relay_namespace()` is removed; tenant tests are updated accordingly; `cargo test -p pikahut` passes.

3. Remove relay namespace metadata from shared-capable scenarios.
Acceptance criteria: `openclaw.rs` and `interop.rs` no longer emit `tenant_relay_namespace`; `tenant_moq_namespace` remains.

4. Update guardrails for moq-only namespace enforcement.
Acceptance criteria: `shared_capable_scenarios_use_tenant_namespace_helpers()` requires `.moq_namespace(` and no longer requires `.relay_namespace(`.

5. Define strict-vs-shared capability matrix for selectors/profiles.
Acceptance criteria: one checked-in matrix marks each target as `SharedSupported`, `StrictOnly`, or `Experimental`; no implicit status.

6. Add lifecycle policy to fixture spec.
Acceptance criteria: `FixtureLifecycle::{PerTest,SharedPerSuite}` exists, `FixtureSpec` supports lifecycle selection, and defaults are documented/tested.

7. Add shared infra pool module.
Acceptance criteria: `crates/pikahut/src/testing/pool.rs` defines `SharedInfra` and a concurrency-safe singleton (`tokio::sync::OnceCell`) for shared infra handles.

8. Implement shared relay and postgres startup paths.
Acceptance criteria: first shared-mode caller initializes relay/postgres once per test process; subsequent callers reuse; tests validate reuse behavior.

9. Implement optional shared MoQ startup path.
Acceptance criteria: MoQ is started/shared only when required by profile; non-MoQ profiles do not start it.

10. Implement `DatabaseHandle` with deterministic tenant lifecycle.
Acceptance criteria: per-test handle creates tenant DB (`CREATE DATABASE`) and drops it reliably (`DROP DATABASE`); behavior is idempotent under retries.

11. Thread lifecycle and pool behavior through fixture startup.
Acceptance criteria: `SharedPerSuite` paths use pool-backed infra; `PerTest` paths preserve current behavior; teardown semantics are correct for both.

12. Add shared-fixture runtime toggles.
Acceptance criteria: `PIKAHUT_SHARED_FIXTURES=1` enables shared mode, `PIKAHUT_STRICT_MODE=1` forces strict mode, and precedence is documented/tested.

13. Add canonical strict-mode deterministic validation command path.
Acceptance criteria: reproducible strict command target exists and is documented for local and CI-like runs.

14. Add canonical shared-mode deterministic validation command path.
Acceptance criteria: reproducible shared command target exists and is documented for local and CI-like runs.

15. Add strict-vs-shared parity summary output and artifact contract.
Acceptance criteria: runs emit comparable strict/shared outcome summaries and preserve actionable logs/metadata.

16. Add relay/moq isolation regression tests under shared mode.
Acceptance criteria: concurrent tenants cannot collide or observe each other in shared relay/moq paths.

17. Add postgres default-mode isolation regression tests.
Acceptance criteria: concurrent tenants cannot read/write across tenant boundaries in default DB lifecycle.

18. Add fallback-mode isolation and deterministic trigger tests.
Acceptance criteria: fallback behavior is deterministic, preserves isolation, and emits diagnostics describing why fallback occurred.

19. Add transient init recovery and irrecoverable failure policy tests.
Acceptance criteria: transient shared-pool init failures recover on retry; irrecoverable failures fail fast with clear surfaced errors.

20. Add teardown resilience tests.
Acceptance criteria: shared teardown handles contention with bounded retry/backoff and remains idempotent.

21. Add drift guardrails for shared-fixture contracts.
Acceptance criteria: guardrails detect docs/toggle/selector divergence and enforce canonical helper usage in shared-capable internals.

22. Wire deterministic lane contracts with strict default plus explicit shared validation.
Acceptance criteria: CI defaults remain strict unless promoted; shared validation lanes are explicit and evidence-producing.

23. Define and enforce promotion evidence contract.
Acceptance criteria: promotion requires parity pass, isolation regressions passing, reliability signal, and runtime delta evidence recorded per lane/profile.

24. Promote shared defaults only where evidence exists.
Acceptance criteria: each promoted lane/profile includes linked evidence artifacts; non-promoted lanes remain strict with documented rationale.

25. Consolidate shared-fixture docs around this canonical todo.
Acceptance criteria: docs reference this plan and current status without conflicting claims or stale completion language.

26. Manual QA gate (user-run).
Acceptance criteria: user runs representative deterministic flows in strict and shared modes, confirms no cross-test contamination, acceptable reliability, and records PASS/FAIL sign-off.
