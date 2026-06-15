# GWS Implementation Plan

Status: planned

Source design: `GWSDesign.md`.

This plan builds two independent repositories:

```text
gws-core
  Rust library crate. Owns model, artifacts, protocol, operations, Git backend,
  runtime, and tests.

gws-cli
  Rust CLI crate. Parses argv, builds taut/GWS requests, calls `gws-core`, and
  renders responses/events.
```

For v0 development, `gws-cli` uses a local path dependency on `../gws-core`.
`gws-core` must not depend on `gws-cli`.

Step size target: each implementation step should stay around 500 hand-written
LOC or less. Generated protocol code, golden fixtures, copied ignore files, and
mechanical repetitions are excluded from that advisory budget.

Accepted implementation decisions:

- V0 runtime uses synchronous `GitBackend` calls on a std thread based worker
  pool. No async runtime is required.
- V0 event delivery uses bounded in-memory buffers. Event emission must not
  deadlock if a caller waits for the final result without subscribing. On
  overflow, older buffered incremental events are dropped, reset state is
  recorded, and the final result is retained separately.
- V0 selects the Git backend only after a capability spike proves clone, fetch,
  fast-forward, checkout, status, and push against local fixtures.
- V0 uses generated Rust protocol types from taut. Hand-maintained shadow
  protocol structs are not allowed.
- V0 artifact filenames follow `GWSDesign.md`: `workspace/gws.yml` and
  `workspace/gws.lock.yml`. Older flat filename text is superseded.
- V0 Rust toolchain is pinned to Rust `1.95.0`, matching the active local Rust
  workstream repos.

## Step 0: Repo Hygiene And Empty Repo Setup

Scope:

- Create the independent `gws-cli` repository.
- Add `.gitignore` to both `gws-core` and `gws-cli`.
- Ignore `scratch/` in both repositories.
- Match the local Rust repo ignore conventions:
  - `.DS_Store`
  - Python/cache artifacts
  - Cargo `target/`
  - Bazel/Razel convenience symlinks and caches
  - compiled Rust leftovers such as `*.rlib`

Deliverables:

- `gws-core/.gitignore`
- `gws-cli/.gitignore`
- Empty initialized `gws-cli` Git repository

Acceptance:

- `git -C gws-core status --short` shows the new ignore file and docs only.
- `git -C gws-cli status --short` shows only intended starter files.
- `scratch/` is ignored in both repos.

## Step 1: Rust Crate Bootstrap

Scope:

- Add minimal Rust project scaffolding to `gws-core`.
- Add minimal Rust project scaffolding to `gws-cli`.
- Keep both repos buildable before any real behavior exists.

Deliverables:

- `gws-core/Cargo.toml`
- `gws-core/Cargo.lock`
- `gws-core/src/lib.rs`
- `gws-core/src/runtime/clock.rs`
- `gws-core/src/runtime/ids.rs`
- `gws-core/README.md`
- `gws-core/AGENTS.md`
- `gws-core/rust-toolchain.toml`
- `gws-cli/Cargo.toml`
- `gws-cli/Cargo.lock`
- `gws-cli/src/main.rs`
- `gws-cli/README.md`
- `gws-cli/AGENTS.md`
- `gws-cli/rust-toolchain.toml`

Implementation notes:

- `gws-core` should expose a tiny placeholder API such as `version()`.
- Add minimal injectable clock/id-provider traits early so protocol corpus and
  artifact golden tests can be deterministic.
- `gws-cli` should depend on `gws-core` through `path = "../gws-core"` for local
  development.
- Pin both repos to Rust `1.95.0`.
- Do not add Git behavior yet.
- Do not add a workspace spanning both repos.

Acceptance:

- Both repos include `rust-toolchain.toml` pinning Rust `1.95.0`.
- `cargo test` passes in `gws-core`.
- `cargo test` passes in `gws-cli`.
- `cargo run -- --version` works in `gws-cli`.

## Step 2: Protocol Seed And Taut Corpus

Scope:

- Promote the accepted protocol sketch into a real taut schema source.
- Generate the first Rust protocol types from taut.
- Add protocol round-trip tests before operations use the messages.

Deliverables:

- `gws-core/protocol/gws.taut.py`
- generated Rust protocol module under `gws-core/src/protocol/`
- a generated-code staleness test that fails when generated Rust drifts from
  `gws.taut.py`
- minimal golden corpus for:
  - `StatusRequest`
  - `ResponseEnvelope`
  - `OperationEvent`
  - `OperationAttribution`
  - one representative per-member error

Implementation notes:

- Taut remains the protocol authority.
- If the Rust generator lacks a needed shape, fix the generator or narrow the
  schema shape; do not hand-edit generated protocol output.
- Generated code size does not count toward the step LOC target.

Acceptance:

- Taut schema loads successfully.
- Rust protocol serialization tests pass.
- Attribution fields round-trip.
- Error enum numeric values are pinned by tests.

## Step 3: Core Model Types

Scope:

- Implement pure model types with no filesystem or Git access.

Deliverables:

- IDs:
  - `WorkspaceId`
  - `SourceId`
  - `MemberId`
  - `OperationId`
- workspace/member/source structs
- remote specs
- desired refs, including `git_tag`
- attribution structs
- selection structs
- policy structs
- resolved member state structs
- typed error enum

Implementation notes:

- Keep model types serializable only where artifact/protocol layers need it.
- Avoid leaking generated protocol types into core planning code.
- Add conversions in a small `protocol::convert` module instead of scattering
  mapping code.

Acceptance:

- Unit tests cover id parsing/display.
- Unit tests cover desired-ref validation.
- Unit tests cover attribution validation.
- Unit tests cover duplicate remote-name rejection.

## Step 4: Path And Workspace Discovery

Scope:

- Implement path validation and workspace discovery.

Deliverables:

- path validator for member paths
- reserved-prefix checks for `workspace/` and `.gws/`
- upward discovery for `workspace/gws.yml`
- init-target handling that does not use upward discovery
- nested-active-workspace rejection for active member roots that contain their
  own `workspace/gws.yml`

Implementation notes:

- Path logic must be testable without touching Git.
- Keep root-relative path normalization explicit.

Acceptance:

- Tests reject absolute paths.
- Tests reject `..` escapes.
- Tests reject `workspace/` and `.gws/`.
- Tests reject path collisions.
- Tests prove `gws` run inside a member resolves to the workspace root.
- Tests prove `gws init` targets the requested/current directory.
- Tests reject adding or validating an active member root that contains its own
  `workspace/gws.yml`.

## Step 5: Artifact I/O

Scope:

- Implement deterministic read/write for GWS-owned files.

Deliverables:

- manifest parser/writer for `workspace/gws.yml`
- lock parser/writer for `workspace/gws.lock.yml`
- snapshot parser/writer for `.gws/snapshots/<id>.yaml`
- GWS tag parser/writer for `workspace/tags/<name>.yml`
- atomic write helper
- golden artifact fixtures

Implementation notes:

- Use structured YAML serialization rather than ad hoc text construction.
- Keep ordering deterministic for useful diffs.
- Manifest `remotes` must use the deterministic list form from
  `GWSDesign.md`; duplicate remote names must fail validation.
- Include `created_by` in snapshot/tag artifacts.
- Schema major versions are parsed from strings like `gws.workspace/v0`; the
  integer after `/v` is the artifact major version. Protocol `schema_version`
  is versioned independently from artifact schemas.

Acceptance:

- Round-trip tests for each artifact type.
- Golden fixture tests for stable output.
- Unsupported major schema versions fail with typed errors.
- Atomic write tests cover success and replacement behavior where practical.

## Step 6: Runtime And Operation Framework

Scope:

- Implement the shared operation skeleton, public dispatch API, and v0 threading
  model before implementing specific operations.

Deliverables:

- `submit(request)`
- `subscribe(operation_id)`
- `wait(operation_id)`
- protocol dispatcher from generated request types to operation specs
- operation registry
- operation context
- operation plan/member plan structs
- preflight report structs
- execution report structs
- bounded in-memory event buffer
- std thread based background executor
- operation result assembly
- per-member mutation lock manager

Implementation notes:

- The backend trait is synchronous; do not introduce `async fn` or require
  Tokio for v0.
- Event emission must not block member execution indefinitely. Buffer overflow
  must drop older buffered incremental events, retain/reset overflow state, and
  preserve the final result.
- `wait()` must be safe to call without draining `subscribe()`.
- V0 uses taut for message types and service shape, not as an RPC transport.
- Read-only operations must not require member mutation locks.
- Mutating operations must lock per member during execution.
- If this step grows past the review budget, split it into:
  - 6A: registry, context, plans, reports, and result assembly
  - 6B: event channels, member locks, execution scheduling, and attribution
    propagation

Acceptance:

- `submit()` returns accepted before a long-running operation finishes.
- A caller can receive accepted response, operation events, and final
  `OperationResult`.
- `wait()` completes even when no subscriber drains events.
- Event-buffer overflow produces reset/overflow state and still preserves the
  final `OperationResult`.
- Dry-run plan tests pass.
- Event sequence numbers are monotonic per operation.
- Per-member event ordering is preserved.
- Parallel members can interleave events.
- Attribution propagates to response, events, and final result.

## Step 7: Git Backend Capability Spike And Implementation

Scope:

- Prove and select the v0 Git backend, then implement it behind the internal
  `GitBackend` trait.

Deliverables:

- backend spike harness
- `GitBackend` trait
- selected backend module, using gix if the spike passes or git2 if gix cannot
  satisfy the v0 surface cleanly
- temporary repository test fixtures
- temporary bare repository fixtures
- support for:
  - repository detection
  - clone from a local or remote Git URL into a member path
  - fetch
  - fast-forward
  - checkout commit
  - push
  - current branch/detached state
  - HEAD commit
  - remotes
  - dirty/untracked/staged/unstaged status
  - create ordinary non-bare repository
  - add remote

Implementation notes:

- Do not use shell `git` as the primary implementation.
- Run the spike before committing the implementation path. The spike must cover
  clone, fetch, fast-forward, checkout, status, and push with local fixtures.
- The spike passes only if the candidate backend handles all required v0 Git
  behavior without shelling out as the primary implementation.
- If gix lacks one behavior needed immediately, use git2 behind the trait for
  v0 or isolate a narrow fallback behind the trait and document it.
- Git object identity plumbing should be present in trait signatures before
  object-creating operations are added.
- No v0 operation writes a Git object. `GitObjectIdentity` is carried and
  validated in v0, but backend consumption waits for commit/merge/annotated-tag
  operations.
- Clone tests should use local temporary repositories/URLs; network clone tests
  are not part of v0 acceptance.

Acceptance:

- Backend spike result and selected v0 backend are recorded in
  `dev-docs/GWSGitBackendDecision.md`.
- Clone into an empty member path succeeds from a local temporary repository.
- Clone into a non-empty path fails before mutation.
- Temp repo status tests cover clean, dirty, staged, unstaged, and untracked.
- Create-repo tests produce ordinary non-bare repositories.
- Remote read/add tests pass.
- Push to a temporary bare repository passes.
- Backend tests do not require network access.

## Step 8: Status Operation

Scope:

- Implement the first end-to-end operation as read-only behavior.

Deliverables:

- `StatusRequest` handling
- selection resolution for all active members, member ids, and paths
- per-member status responses
- lock-match calculation
- aggregate status calculation

Implementation notes:

- This is the first proof that protocol, artifacts, model, Git backend, and
  operation framework fit together.
- Keep status read-only.

Acceptance:

- Status on empty workspace succeeds.
- Status on clean member reports clean Git status.
- Status on dirty member reports dirty counts.
- Unknown/inactive/ambiguous selection fails before member work.
- JSON/protocol response includes one per-member response per selected member.

## Step 9: Workspace Creation Operations

Scope:

- Implement operations that create or register workspace members.

Deliverables:

- `CreateWorkspaceRequest`
- `InitFromSourcesRequest` planning and validation
- `CreateRepoRequest`
- `AddExistingRepoRequest`
- default path derivation as `repos/<repo-name>`
- lock writes according to artifact write policy

Implementation notes:

- `InitFromSourcesRequest` may plan clone/materialize behavior before full
  network clone support exists; unsupported execution can be typed if needed.
- Full `InitFromSourcesRequest` execution for the default head target is
  completed in Step 12, after clone and materialize-to-head are green.
- Add-existing should not reclone.
- Create-repo should mark local-only desired state.
- If this step grows past the review budget, split it into:
  - 9A: create workspace and create local repository
  - 9B: add existing repository and init-from-sources planning

Acceptance:

- Create workspace rejects existing workspace.
- Create workspace rejects nested active workspace.
- Init path derivation is deterministic and collision-safe.
- Add existing repo records current branch, commit, remotes, and dirty state.
- Create repo writes manifest and lock.
- Create repo with no commits records no commit in the lock and materializes as
  `noop` until a commit exists.

## Step 10: Snapshot And GWS Tag Operations

Scope:

- Implement GWS-owned snapshot and tag records.

Deliverables:

- `SnapshotRequest`
- `TagRequest`
- snapshot artifact writes
- GWS tag artifact writes
- materialize-target lookup helpers for snapshot/tag records

Implementation notes:

- GWS tags must not create Git tags.
- Snapshot and tag operations do not rewrite the lock.
- Include selected member ids and resolved member states.

Acceptance:

- Snapshot writes `.gws/snapshots/<id>.yaml`.
- Tag writes `workspace/tags/<name>.yml`.
- Duplicate or invalid tag names fail cleanly.
- Snapshot/tag responses include per-member results.
- `created_by` attribution is written when supplied.

## Step 11: Materialize To Lock, Snapshot, And Tag

Scope:

- Implement materialization to already-recorded resolved states.

Deliverables:

- `MaterializeRequest` for `lock`
- `MaterializeRequest` for `snapshot`
- `MaterializeRequest` for `tag`
- `PullSnapshotRequest`
- clone-missing-member path for materialize-to-lock
- checkout planning for exact commits
- clean-worktree preflight
- destructive-policy gate

Implementation notes:

- Materialize-to-lock reads the lock and does not rewrite it by default.
- Materialize-to-lock clones missing members before checking out locked commits.
- Materialize-to-snapshot/tag updates the lock after success because the current
  materialized workspace changed.
- Pull-to-snapshot is a thin operation wrapper over the same exact-commit
  planner as materialize-to-snapshot, but it writes the lock after success.
- Destructive checkout requires explicit policy.
- If this step grows past the review budget, split it into:
  - 11A: materialize-to-lock with clone-missing support
  - 11B: materialize-to-snapshot/tag and pull-to-snapshot

Acceptance:

- Missing lock/snapshot/tag fails before mutation.
- Missing member materialize-to-lock clones the member and checks out the locked
  commit.
- Dirty selected member blocks default materialization.
- Clean member can move to exact recorded commit.
- Successful snapshot/tag materialization rewrites lock.
- Successful pull-to-snapshot rewrites lock.
- Dry-run returns planned checkout actions without mutation.

## Step 12: Pull To Head

Scope:

- Implement remote-backed fast-forward updates.

Deliverables:

- fetch support in Git backend
- fast-forward support in Git backend
- `PullHeadRequest`
- `MaterializeRequest` for `head`
- complete `InitFromSourcesRequest` execution for local Git URLs using the
  default head target
- missing remote and diverged-member preflight errors

Implementation notes:

- Default behavior is fast-forward only.
- Local-only members return `noop`.
- Rebase, merge, reset, and partial behavior stay explicit policy choices.

Acceptance:

- Local-only pull returns per-member `noop`.
- Clean fast-forward member updates.
- Successful pull-to-head rewrites `workspace/gws.lock.yml`.
- `gws init <local-url>...` can create a workspace, clone members to head, and
  write the initial lock.
- Dirty member blocks whole operation before mutation.
- Diverged member blocks whole operation before mutation.
- All-selected atomic behavior leaves members unchanged when one cannot update.

## Step 13: Push Operation

Scope:

- Implement selected member push behavior with per-member outcomes.

Deliverables:

- push support in Git backend
- `PushRequest`
- remote/refspec policy handling
- credential-ref pass-through in operation attribution/context

Implementation notes:

- GWS Core passes credential references; it does not own credential storage.
- Push does not write GWS artifacts by default.
- Remote rejection must map to a typed per-member result.

Acceptance:

- Missing push remote fails or skips according to policy.
- Local-only member without remote follows policy.
- Remote rejection is reported per member.
- Aggregate status reflects all member outcomes.
- Tests use local temporary bare repositories, not network services.

## Step 14: CLI Parser And Request Builder

Scope:

- Implement `gws-cli` command parsing and request construction.

Deliverables:

- command parser
- request-id creation
- workspace discovery integration
- command mapping:
  - `gws init`
  - `gws init <url>...`
  - `gws add <repo-path>`
  - `gws repo create <member-path>`
  - `gws materialize --lock|--head|--snapshot|--tag`
  - `gws pull --head|--snapshot`
  - `gws snapshot <name>`
  - `gws tag <name>`
  - `gws push`
  - `gws status`
- global flags:
  - `--root`
  - `--member`
  - `--path`
  - `--all`
  - `--dry-run`
  - `--partial`
  - `--force`
  - `--sync`
  - `--remote`
  - `--jobs`
  - `--json`
  - `--jsonl`

Implementation notes:

- CLI must not call Git directly.
- CLI must not read or write GWS artifacts directly.
- CLI local attribution fallback may use local Git config only if explicitly
  documented in the driver.

Acceptance:

- Parser unit tests assert argv-to-request mappings.
- Invalid command combinations fail before core execution.
- CLI can call in-process `gws-core`.

## Step 15: CLI Renderers

Scope:

- Implement human, JSON, and JSONL renderers.

Deliverables:

- human response renderer
- human event renderer
- JSON final response/result renderer
- JSONL response/event/result stream renderer
- stable exit-code mapping

Implementation notes:

- Human output can evolve, but JSON/JSONL should remain protocol-shaped.
- JSONL is the primary integration-test surface.

Acceptance:

- JSON renderer tests compare structured output.
- JSONL renderer emits immediate response, events, and final result in order.
- Human renderer smoke tests cover success, rejection, and per-member failure.
- Exit codes distinguish success, rejected request, and execution failure.

## Step 16: End-To-End Local Integration Tests

Scope:

- Add integration tests that exercise both repositories together.

Deliverables:

- temp workspace fixtures
- temp local Git remotes
- CLI integration tests
- core operation integration tests

Implementation notes:

- Tests should avoid network access.
- Prefer JSONL assertions over human output assertions.
- Keep fixtures small and deterministic.

Acceptance:

- `gws init <local-url>... --jsonl` works.
- fresh checkout with committed `workspace/gws.yml` and `workspace/gws.lock.yml`
  can run `gws materialize --lock` and clone all missing members at locked
  commits.
- `gws status --json` works from workspace root and inside a member.
- `gws snapshot NAME` and `gws tag NAME` write expected artifacts.
- `gws pull --head` fast-forwards clean local remote fixtures.
- `gws push --remote <name>` pushes to a temporary bare repository.
- `gws materialize --snapshot NAME` and `gws pull --snapshot NAME` work against
  local fixtures.
- `gws add <repo-path>`, `gws repo create <member-path>`, and representative
  `--dry-run` commands have CLI integration coverage.
- Failure scenario proves atomic preflight prevents partial mutation.

## Step 17: Documentation And Bootstrap Cut

Scope:

- Document the first usable local workflow and remaining deferred work.

Deliverables:

- `gws-core/README.md` updated with library scope and test commands.
- `gws-cli/README.md` updated with CLI examples.
- short protocol/codegen note.
- known deferrals list:
  - source catalog persistence
  - archive/package/local/generated materialization
  - file watching
  - branch/merge selection
  - alternate Git storage backends
  - remote capability enforcement
  - persistent `.gws/operations/<operation-id>.jsonl` event logs

Acceptance:

- A new contributor can run tests in both repos from README instructions.
- The accepted design and implementation plan agree on v0 operation scope.
- Deferred items are explicit and not hidden as TODOs inside code.
