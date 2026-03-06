## Spec

Why this is being done:
The current microVM launcher code still reflects an earlier “generic remote dev box” design with
three runtime variants, many request-time knobs, and spawner-owned state that no longer matches the
actual personal-agent product. That complexity is now the main source of drift, review difficulty,
and recovery bugs.

Intent and expected outcome:
Replace the current multi-variant launcher model with one production launcher path that is easy to
reason about: a deterministic, durable-home microVM appliance launched by a tiny privileged
`vm-spawner`, with all lifecycle authority and policy living in `pika-server`.

Exact build target (what will exist when done):
1. One launcher path:
only the durable prebuilt runner path remains; `legacy` and `prebuilt` are fully removed.
2. One product model:
the launcher is treated as an agent appliance bootstrapper, not a generic Nix dev environment or
SSH-first remote shell target.
3. One authoritative control plane:
`pika-server` is the only source of truth for owner -> agent -> vm mapping, desired lifecycle
phase, and policy decisions.
4. One narrow privileged adapter:
`vm-spawner` only performs host-local privileged operations required to create, recover, delete,
and health-check a VM.
5. Deterministic host behavior:
unit name, tap name, MAC address, IP, state paths, and required runtime metadata are derived from
`vm_id` plus host defaults, with compatibility handling for older persisted metadata during
transition.
6. Minimal launcher contract:
create requests contain only guest autostart payload plus any strictly required future-proofed
fields; create/recover responses contain only continuation data the server actually uses.
7. Durable recovery model:
guest state lives under host-backed persistent `/root`; recover means reboot first, then recreate
the VM while reusing the same durable home if reboot fails.
8. Minimal operator surface:
deleted variants and APIs leave behind no dead env vars, Nix options, docs, or tests.

Exact approach (how it will be accomplished technically):
1. Treat the simplification as a new branch that preserves only the good direction from the current
scope-lock work, rather than continuing to pile architecture changes onto an in-flight cleanup
branch.
2. Freeze the desired end state first: one launcher path, one source of truth, one private spawner
API, and one durability model.
3. Add compatibility shims only where needed to safely move existing VMs and hosts onto the
deterministic model.
4. Delete launcher flexibility that is no longer part of the product, especially per-request guest
shape, SSH/session machinery, and informational spawner state surfaces.
5. Keep deterministic coverage focused on the real production flow: `ensure`, `me`, `recover`,
spawner create/recover/delete, and durable state preservation.

## Plan

1. Freeze the target launcher model in docs before changing code.
Acceptance criteria: one short design note states that the supported launcher is a durable
prebuilt-runner appliance, not a generic remote dev VM; it explicitly rejects `legacy` and
`prebuilt` as product paths and names `pika-server` as lifecycle authority.

2. Create the simplification on a fresh branch rather than extending the current cleanup branch.
Acceptance criteria: the work starts from a clean branch whose stated scope is “single launcher
path simplification”; any fixes cherry-picked from the current branch are deliberate and minimal,
and the branch description distinguishes architecture simplification from bug-fix cleanup.

3. Define the one launcher path precisely.
Acceptance criteria: the retained runtime path is documented as:
host-built prebuilt runner + host-backed persistent `/root` + guest autostart payload + reboot then
recreate-with-same-home recovery.
The doc explicitly states that no per-request flake/dev-shell selection is part of the supported
product path.

4. Remove `legacy` launcher code end to end.
Acceptance criteria: `SpawnVariant::Legacy`, legacy flake generation, legacy `microvm create`
logic, legacy SSH bootstrap assumptions, and any tests/docs/config tied only to that path are
deleted.

5. Remove `prebuilt` as a distinct runtime mode.
Acceptance criteria: there is no separate non-durable or fresh-workspace “prebuilt” branch left in
code; the retained path always uses the durable-home semantics that were previously associated with
`prebuilt-cow`.

6. Collapse launcher naming to a single production path.
Acceptance criteria: public and internal code no longer carries variant parsing, `spawn_variant`
request fields, persisted variant fields, or branching by variant name; if a name remains at all,
it is an internal implementation detail with exactly one allowed value.

7. Recast `vm-spawner` as a tiny privileged adapter.
Acceptance criteria: retained endpoints are only:
`GET /healthz`
`POST /vms`
`POST /vms/:id/recover`
`DELETE /vms/:id`
There are no enumeration, inspection, exec, capacity, or public debugging endpoints left in
production code.

8. Move all lifecycle and policy authority to `pika-server`.
Acceptance criteria: `pika-server` remains the only durable owner of owner-to-vm mapping, desired
lifecycle phase, and admission policy; `vm-spawner` is not consulted for authoritative enumeration,
ownership lookup, or app-visible phase truth.

9. Reduce the launcher request contract to true caller inputs only.
Acceptance criteria: create requests keep only guest autostart payload and any field that has both
an active production caller and a documented reason to remain caller-controlled; `flake_ref`,
`dev_shell`, `spawn_variant`, SSH-related inputs, and similar generic-vm knobs are removed.

10. Reduce the launcher response contract to continuation data only.
Acceptance criteria: create/recover responses contain only the fields `pika-server` needs to keep
going, at minimum `id` and optionally a narrow status field; SSH private keys, timing maps, variant
metadata, debug fields, and session-token data are removed.

11. Delete spawner-owned durable registry state.
Acceptance criteria: `vm-spawner` does not maintain an authoritative in-memory VM map loaded from
disk, does not persist `vm.json` or `sessions.json` as lifecycle truth, and does not require
replay-from-disk logic to recover correctness after restart.

12. Keep only host-boot metadata that is necessary to start or recreate the guest.
Acceptance criteria: files under `/var/lib/microvms/<vm_id>` are limited to runtime metadata,
durable home contents, runner symlinks, and other boot inputs required by the retained path; none
of those files are treated as authoritative control-plane records.

13. Make host layout fully deterministic from `vm_id`.
Acceptance criteria: unit name, state dir, gcroot paths, tap name, MAC address, and default guest
IP are derived from `vm_id` plus host defaults; create, recover, and delete can rediscover the
same host artifacts after spawner restart without loading a spawner database.

14. Add explicit compatibility handling for already-created VMs during migration.
Acceptance criteria: recover/delete can safely operate on VMs created before the simplification by
reading prior runtime metadata where necessary; migration behavior is documented, and there is a
clear point at which legacy compatibility can later be removed.

15. Prevent in-flight create races under deterministic allocation.
Acceptance criteria: concurrent create requests cannot allocate the same deterministic IP or host
layout while a prior create is still in progress; tests cover reservation/release behavior for the
smallest IP pool case.

16. Keep the durable-home recovery contract and nothing more.
Acceptance criteria: recover first tries reboot, then recreates the VM with the same persistent
home when reboot fails; the recreated VM rewrites any runtime metadata required by the deterministic
host layout before restart.

17. Remove guest SSH and per-VM key machinery unless a real operator need is documented.
Acceptance criteria: if no current production consumer exists, SSH key generation, authorized-key
injection, SSH response fields, and related docs/tests/config are deleted; if any piece remains, it
has an explicit operator use case and minimal lifecycle design.

18. Remove session-token and LLM-sideband launcher state unless a real runtime consumer exists.
Acceptance criteria: `llm_session_token`, `sessions.json`, and related config/env are removed from
models, persistence, API responses, and docs unless an active production dependency is identified
and documented.

19. Remove dead operator knobs from config and Nix modules.
Acceptance criteria: env vars and Nix options tied only to removed variants, removed APIs, removed
request fields, or removed registry/session state are deleted from `vm-spawner`, infra modules, and
host docs.

20. Keep one guest bootstrap model.
Acceptance criteria: the retained guest bootstrap path uses runtime metadata + autostart payload +
durable `/root`; there is no leftover parallel bootstrap mechanism such as per-request Nix
devshell selection, workspace image mode switching, or variant-specific guest setup.

21. Simplify the crate and type boundaries around the spawner contract.
Acceptance criteria: the runtime path uses one clear location for spawner request/response types
and client helpers; there is no extra crate or abstraction layer that exists only to carry deleted
variant/config fields forward.

22. Map spawner failures to the right boundary semantics.
Acceptance criteria: malformed `vm_id` and missing VM cases return appropriate private API errors
such as `400`/`404` rather than generic `500`; `pika-server` continues to translate those outcomes
into the app-facing `recover_failed`/`internal` contract as appropriate.

23. Keep deterministic tests only around the supported production path.
Acceptance criteria: tests cover:
single-path spawner create payload shape
single-path recover/delete behavior
deterministic host derivation
compatibility recovery for older persisted runtime metadata
concurrent allocation protection
`pika-server` ensure/me/recover flow
Tests for deleted variants, deleted endpoints, SSH/session responses, or generic dev-shell behavior
are removed.

24. Align docs and operational guidance with the simplified launcher model.
Acceptance criteria: docs describe the system as a private personal-agent launcher with durable
state, not a general-purpose remote dev VM platform; backup/restore guidance describes persistent
home as the durable asset, and no docs reference deleted variant names or deleted launcher knobs.

25. Add a guardrail scope-lock doc to prevent complexity from returning.
Acceptance criteria: one short doc enumerates:
allowed spawner responsibilities
allowed private endpoints
allowed request/response/config surface
explicit non-goals such as “no generic dev VM launcher,” “no authoritative lifecycle DB in
vm-spawner,” and “no multiple launcher variants without a separate product decision.”
