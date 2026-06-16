# GWZ Core Requirements

Status: complete

GWZ Core is a standalone library for defining, materializing, observing, and
operating on a workspace made from independently owned sources.

This document is a requirements document. It defines required behavior,
interfaces, constraints, and deferred scope. It does not define the internal
implementation design.

## Requirement Conventions

- `MUST` requirements are mandatory.
- `SHOULD` requirements are expected unless a later decision records why they
  are deferred or changed.
- `MAY` requirements are permitted behavior.
- `v0` means the first implementation target.
- Every requirement intended for v0 MUST be traceable to one or more tests
  before implementation is accepted.

## Goals

- GWZ Core MUST define a workspace containing repositories, packages, archives,
  local sources, and generated members.
- GWZ Core MUST provide APIs for materializing, synchronizing, pushing,
  tagging, snapshotting, and observing workspace members.
- GWZ Core MUST remain independent from any specific CLI, build tool, UI,
  hosted forge, or application runtime.
- GWZ Core MUST support high-concurrency workspace operations while keeping each
  member's mutating operations serialized by default.
- GWZ Core MUST expose a message-oriented API so callers can observe progress
  without blocking on final operation completion.

## Non-Goals

- GWZ Core MUST NOT be a build system.
- GWZ Core MUST NOT be a package registry.
- GWZ Core MUST NOT be a hosted Git service.
- GWZ Core MUST NOT require a daemon.
- GWZ Core MUST NOT require Git submodules.
- GWZ Core MUST NOT own credential storage.
- GWZ Core v0 MUST NOT require a source catalog separate from the workspace
  manifest.

## Terms

- **Workspace**: A local root with a manifest, optional lock, internal state, and
  a set of members.
- **Manifest**: The human-readable workspace intent file.
- **Lock**: The human-readable resolved state file for the current workspace
  materialization.
- **Snapshot**: A named recorded resolved state for a workspace or selection.
- **Source**: An origin of content, such as a Git repository, archive, package,
  local directory, or generated output.
- **Source id**: Stable identity for a logical source.
- **Member**: A source materialized at a path inside a workspace.
- **Member id**: Stable identity for a member.
- **Remote repository**: A named Git endpoint used for fetch or push.
- **Selection**: A resolved set of member ids.
- **Policy**: Caller-supplied or manifest-supplied behavior choices for an
  operation.
- **Operation**: A long-running action such as materialize, pull, push, tag, or
  snapshot.
- **Message**: A taut-defined request, response, event, or result payload.
- **Driver**: An external CLI, daemon, UI, or other system using GWZ Core.

## V0 Scope

### REQ-000: V0 Source Kind

GWZ Core v0 MUST implement Git members.

GWZ Core v0 MUST reject unsupported source kinds during validation unless the
member is explicitly marked inactive.

### REQ-001: V0 Required Operations

GWZ Core v0 MUST support:

- workspace creation
- workspace creation from supplied Git source URLs
- manifest validation
- lock read and write
- snapshot read and write
- add existing Git repository
- create local Git repository
- materialize to lock
- materialize to head
- status
- snapshot current selection
- pull to snapshot
- pull to head
- push selected Git members

### REQ-002: V0 Deferred Operations

GWZ Core v0 MAY defer:

- source catalog persistence
- archive materialization
- package materialization
- local directory materialization
- generated source materialization
- selection-wide branch
- selection-wide merge
- file watching
- bare repository, worktree, and mirror-cache storage backends

Deferred operations MUST either be rejected with typed unsupported-operation
errors or hidden from the v0 action surface.

## System Boundary

### REQ-010: Standalone Library

GWZ Core MUST be usable as a standalone library with no dependency on any
specific driver.

### REQ-011: In-Process Operation

All core operations MUST be callable in-process.

A driver MAY run GWZ Core inside a daemon, but daemon use MUST NOT be required.

### REQ-012: Driver-Owned Policy

GWZ Core MUST expose typed policy inputs for behavior that can vary by driver,
workspace, member, or operation.

GWZ Core MUST NOT hard-code driver-specific policy.

### REQ-013: Capability Gates Deferred

GWZ Core v0 MAY assume local caller authority.

Remote capability enforcement, user consent, and agent permission checks are
deferred to drivers or later requirements.

## Workspace Artifacts

### REQ-020: Manifest Filename

The v0 manifest filename MUST be `workspace.gwz.yaml`.

### REQ-021: Manifest Shape

The v0 manifest MUST use a native GWZ schema.

The manifest MUST keep member identity separate from member materialization
path.

### REQ-022: Manifest Minimum Fields

The manifest MUST record:

- schema version
- workspace id
- member id for each member
- member source kind
- member materialization path
- source id or inline source definition for each member
- desired revision, version, tag, branch, or local-only marker for each member
- remote repositories for Git members when configured
- member active or inactive state

### REQ-023: Lock Filename

The v0 lock filename MUST be `workspace.gwz.lock.yaml`.

### REQ-024: Lock Minimum Fields

The lock MUST record:

- schema version
- workspace id
- manifest schema version used to create the lock
- resolved member state for every locked member
- lock creation timestamp

For Git members, resolved member state MUST include:

- member id
- member path
- source id
- source kind
- commit
- branch or detached state
- remote repository URLs
- upstream tracking state when available
- dirty state at lock time
- materialization state

### REQ-025: Internal State Directory

The internal state directory name MUST be `.gwz`.

### REQ-026: Snapshot Storage

Snapshots MUST be stored under the workspace internal state directory.

Each snapshot MUST have a stable snapshot id.

Each snapshot MUST record:

- schema version
- workspace id
- snapshot id
- selected member ids
- resolved member state for each selected member
- snapshot creation timestamp

### REQ-027: Human-Readable Artifacts

Manifest, lock, and snapshot files MUST be human-readable and diff-friendly.

### REQ-028: Atomic Artifact Writes

Manifest, lock, and snapshot writes MUST be atomic where supported by the host
filesystem.

### REQ-029: Schema Versioning

Manifest, lock, snapshot, and protocol schemas MUST include explicit schema
versions.

Readers MUST reject unsupported major schema versions with typed errors.

## Identity Model

### REQ-030: Workspace Identity

Each workspace MUST have a stable workspace id.

The workspace id MUST be persisted in the manifest.

### REQ-031: Member Identity

Each member MUST have a stable member id.

The member id MUST be persisted in the manifest.

The member id MUST remain stable across member path changes and remote URL
changes.

### REQ-032: Member Id Assignment

GWZ Core MUST assign a member id when creating a member, adding an existing
repository, or loading a manifest entry that lacks a member id.

Generated member ids MUST be persisted on the next manifest write.

### REQ-033: Source Identity

Each member MUST refer to a source id or contain an inline source definition.

GWZ Core v0 MAY use the same value for source id and member id when no separate
source catalog exists.

### REQ-034: Source To Member Relationship

One source MAY be materialized as more than one member across one or more
workspaces.

One member MUST refer to exactly one source.

## Source And Member Model

### REQ-040: Source Kind

Each source MUST have an explicit source kind.

### REQ-041: Local-First Source

GWZ Core MUST support Git sources that exist locally before they have any remote
repository.

### REQ-042: Source Catalog Deferral

A source catalog records known sources independently from any one workspace.

GWZ Core v0 MUST NOT require a source catalog.

If a source catalog is added later, the workspace manifest MUST remain the
authority for workspace membership.

### REQ-043: Path Safety

Member paths MUST be relative to the workspace root.

GWZ Core MUST reject member paths that:

- are absolute
- escape the workspace root
- collide with another member path
- collide with the internal state directory

### REQ-044: Nested Workspace Policy

GWZ Core v0 MUST reject nested active GWZ workspaces.

A member MAY contain unrelated repository or package files.

## Selection Model

### REQ-050: Selection Inputs

Operations that accept a selection MUST support whole-workspace selection.

Operations SHOULD support ad hoc selection by member id and member path.

Named selections MAY be added after v0.

### REQ-051: Selection Resolution

Selections MUST resolve to a deterministic set of member ids before operation
preflight begins.

Selection resolution MUST fail with a typed error if it references unknown,
inactive, or ambiguous members.

### REQ-052: Whole Workspace Default

When an operation does not provide a selection, the default selection MUST be all
active workspace members.

### REQ-053: Selection Traceability

Operation responses and events MUST report the resolved member ids included in
the selection.

## Policy Model

### REQ-060: Policy Scopes

Policies MUST be representable at these scopes:

- workspace default
- member override
- operation request override

Operation request policy MUST take precedence over member policy.

Member policy MUST take precedence over workspace policy.

### REQ-061: Atomic Mutation Default

Selection-wide mutating operations MUST preflight every selected member before
changing any selected member.

By default, if any selected member cannot perform the requested operation
cleanly, the whole operation MUST be rejected before mutation.

### REQ-062: Partial Mode

Callers MAY explicitly request partial or best-effort behavior.

Partial mode MUST be visible in the request and in the final result.

### REQ-063: Destructive Policy

Operations MUST NOT discard local changes unless the request explicitly selects
a destructive policy.

### REQ-064: Policy Values

GWZ Core MUST define policy values for:

- sync behavior
- destructive behavior
- partial behavior
- tag behavior
- unsupported member behavior

## Git Requirements

### REQ-070: Local Repository Creation

GWZ Core MUST provide an API to create a local Git source without a remote
repository.

### REQ-071: Add Existing Repository

GWZ Core MUST support adding an existing Git repository to a workspace as a
member without recloning it.

The add operation MUST record current branch or detached state, current commit,
configured remotes, and dirty state.

### REQ-072: Remote Attachment

GWZ Core MUST support attaching a remote repository to a local Git source after
source creation.

### REQ-073: Multiple Remote Repositories

GWZ Core MUST allow a Git source to define multiple named remote repositories.

Fetch, push, and pull operations MUST allow caller policy to select the remote
repository to use.

### REQ-074: Git Storage Backend

GWZ Core v0 MUST materialize Git members as ordinary non-bare working
repositories.

Public APIs MUST NOT require callers to depend on the Git storage backend.

### REQ-075: Submodule Policy

GWZ Core MUST NOT require Git submodules to represent workspace members.

GWZ Core MAY allow caller policy to configure member-local submodule behavior.

### REQ-076: Git Status

Git member status MUST include:

- branch or detached state
- HEAD commit
- configured upstream when available
- ahead and behind counts when available
- staged change count
- unstaged change count
- untracked count
- dirty state

### REQ-077: Local-Only Pull

A Git member explicitly marked local-only MUST return `noop` for pull-to-head.

A Git member expected to track a remote but missing required remote
configuration MUST fail preflight.

## Operation Requirements

### REQ-080: Operation Taxonomy

GWZ Core MUST define one taut action for each supported operation.

The v0 action set MUST include:

- create workspace
- create workspace from sources
- add existing repository
- create repository
- materialize workspace or selection
- status
- snapshot
- pull to head
- pull to snapshot
- push

### REQ-081: Materialize

GWZ Core MUST support materializing a workspace or selection from a manifest and
optional lock.

Materialization MUST support these targets:

- lock
- head
- snapshot

Materialization MAY support tag targets.

### REQ-081A: Initialize From Source URLs

GWZ Core MUST support creating a workspace from a supplied ordered list of Git
source URLs.

Initialization from source URLs MUST:

- create workspace artifacts
- create one active Git member per supplied URL
- derive a default member path when a path is not provided
- resolve each member to the requested target
- materialize all selected members
- write a lock recording the resolved commits
- return aggregate status and per-member results

The default target for initialization from source URLs SHOULD be head.

Initialization from source URLs MUST fail before mutation when input validation
fails.

### REQ-082: Pull To Head

Pull-to-head MUST be a selection-wide operation that moves selected Git members
toward configured heads.

By default, pull-to-head MUST use fast-forward-only behavior for Git members.

By default, pull-to-head MUST reject the whole operation before mutation if any
selected member is dirty, diverged, missing required remote information, or
unable to update cleanly.

Rebase, merge, reset, and partial update behavior MUST require explicit caller
policy.

### REQ-083: Pull To Snapshot

Pull-to-snapshot MUST be a selection-wide operation that moves selected members
to exact resolved states recorded in a snapshot or lock.

Pull-to-snapshot MUST reject the whole operation before mutation if any selected
member has dirty local state or cannot move to the requested state cleanly.

Discarding local changes during pull-to-snapshot MUST require explicit
destructive policy.

### REQ-084: Snapshot

GWZ Core MUST support snapshotting the current state of a workspace or
selection.

A snapshot MUST record enough resolved member state to later materialize the
same selected members again.

### REQ-085: Push

GWZ Core MUST support pushing selected push-capable Git members according to
caller policy.

Push responses MUST report per-member success, rejection, skipped, and failed
states.

### REQ-086: Tag

GWZ Core SHOULD support applying a Git tag to selected tag-capable Git members.

Tag operations MUST fail preflight when selected members are not tag-capable
unless caller policy explicitly permits skipping unsupported members.

GWZ Core tag operations SHOULD create annotated Git tags by default.

Lightweight Git tags MAY be requested explicitly.

### REQ-087: Workspace Snapshot Vs Git Tag

A GWZ snapshot and a Git tag MUST be distinct concepts.

Snapshots MUST be stored by GWZ Core.

Git tags MUST be stored in Git repositories.

### REQ-088: Branch Selection

Selection-wide branch operations SHOULD be supported after v0.

Branch operations MUST reject the whole operation before mutation if any selected
member cannot branch cleanly, unless explicit partial policy is requested.

### REQ-089: Merge Selection

Selection-wide merge operations SHOULD be supported after v0.

Merge operations MUST reject the whole operation before mutation if any selected
member cannot merge cleanly, unless explicit partial policy is requested.

### REQ-089A: Compare To Snapshot

GWZ Core SHOULD support comparing current workspace or selection state to a
snapshot.

## Message Protocol Requirements

### REQ-090: Taut Authority

GWZ Core API messages MUST be defined using taut schemas.

Generated language-specific APIs MAY wrap generated message types, but taut
schemas MUST remain the protocol authority.

### REQ-091: Request Envelope

Every action request MUST include:

- request id
- schema version
- action kind
- workspace reference when the action targets an existing workspace
- selection when the action targets members
- policy when policy differs from defaults
- dry-run flag for mutating operations

### REQ-092: Response Envelope

Every action response MUST include:

- request id
- schema version
- aggregate status
- operation id when the action was accepted as a long-running operation
- per-member responses when the action targets a selection
- errors when the action is rejected

### REQ-093: Per-Member Response

Per-member responses MUST include:

- member id
- member path
- source kind
- status
- error code when applicable
- human-readable message when applicable
- planned change when applicable
- resulting revision or snapshot state when applicable

### REQ-094: Operation Event

Long-running operations MUST emit structured operation events.

Operation events MUST include:

- operation id
- monotonically increasing sequence number within the operation
- event kind
- timestamp
- member id when applicable
- severity
- machine-readable error code when applicable
- human-readable message when applicable

### REQ-095: Operation Result

Long-running operations MUST produce a final operation result.

The final result MUST include aggregate status and per-member results for every
selected member.

### REQ-096: Event Ordering

GWZ Core MUST preserve event ordering within a single member operation.

GWZ Core MAY interleave events from different members.

### REQ-097: Backpressure

Event streams SHOULD support bounded buffering or backpressure.

## Concurrency Requirements

### REQ-100: Parallel Member Operations

GWZ Core MUST be able to run operations across multiple selected members
concurrently.

### REQ-101: Per-Member Serialization

GWZ Core MUST serialize mutating operations per member by default.

### REQ-102: Configurable Concurrency

Workspace operations SHOULD support configurable concurrency limits.

### REQ-103: Non-Blocking Progress

Callers MUST be able to receive progress, output, and per-member completion
events without waiting for the full workspace operation to finish.

## Status And Observation Requirements

### REQ-110: Workspace Status

GWZ Core MUST provide workspace status.

### REQ-111: Member Status

Member status MUST include:

- member id
- path
- source kind
- active state
- materialization state
- lock match state
- dirty state when available
- errors

### REQ-112: File Change Observation

File change observation SHOULD be supported after v0.

When file change observation is supported, it MUST define watched path scope,
ignore behavior, coalescing behavior, and reset events.

### REQ-113: Reset Events

Watch APIs MUST be able to emit reset events when a watcher overflows, loses
state, or cannot produce reliable incremental deltas.

## Error Requirements

### REQ-120: Typed Errors

GWZ Core MUST expose typed errors suitable for programmatic handling.

### REQ-121: Error Code Registry

Machine-readable error codes MUST be defined in a versioned taut enum.

The same error code values MUST be used in responses, operation events, and
operation results.

### REQ-122: External Tool Errors

If an operation uses an external tool, GWZ Core MUST map relevant tool failures
to typed GWZ error codes.

### REQ-123: Recoverable Errors

Default atomic mutating operations MUST stop before mutation when preflight
finds recoverable per-member errors.

For explicit partial operations and read-only operations, per-member failures
SHOULD NOT stop unrelated members unless caller policy requires fail-fast
behavior.

### REQ-124: Credentials

Credential acquisition MUST be delegated to caller policy, host configuration, or
explicit adapter APIs.

## Deferred Requirements

### REQ-140: Source Catalog

A separate source catalog MAY be added after v0.

If added, a source catalog MUST NOT silently override workspace manifest
membership.

### REQ-141: Non-Git Materialization

Archive, package, local directory, and generated source materialization MAY be
added after v0.

### REQ-142: Alternate Git Storage

Bare repositories, worktrees, and local mirror caches MAY be added after v0 as
storage backends.

Public APIs MUST remain independent from storage backend choice.

## Testability Requirements

### REQ-150: In-Memory Tests

Manifest, lock, snapshot, identity, selection, policy, and operation planning
logic MUST be testable without network access.

### REQ-151: Temporary Repository Tests

Git behavior SHOULD be testable using temporary local repositories.

### REQ-152: Deterministic Serialization

Manifest, lock, snapshot, and message serialization SHOULD have deterministic
ordering for stable tests and useful diffs.

### REQ-153: Requirement Traceability

Every v0 requirement MUST have at least one test reference before the
implementation is accepted.
