# gws-core

GWS Core is the Rust library for defining, materializing, observing, and
operating on a multi-repository GWS workspace.

GWS Core owns workspace artifacts, protocol types, operation handlers, Git
backend behavior, and local operation tests. It does not own the command-line
interface; `gws-cli` is the thin driver that builds requests and renders
responses.

The accepted design and implementation plan live in `dev-docs/`.

## Current Scope

- Workspace artifact I/O for `workspace/gws.yml`, `workspace/gws.lock.yml`,
  `.gws/snapshots/<id>.yaml`, and `workspace/tags/<name>.yml`.
- Git-backed workspace operations for status, workspace creation, add existing
  repo, create repo, init from source URLs, materialize, snapshot, tag, pull,
  and push.
- Generated taut protocol types and CBOR round-trip tests.
- Local Git fixtures for clone, fetch, fast-forward, checkout, status, and
  push behavior.

## Development

```text
cargo fmt
cargo test
cargo fmt --check
```

## Protocol Codegen

The taut schema is `protocol/gws.taut.py`. Generated Rust protocol files live
under `src/protocol/` and are checked by the protocol staleness test in
`tests/protocol.rs`.

When the schema changes, regenerate through the existing taut workflow and run
`cargo test`; do not hand-edit generated protocol output.

## Deferred Work

- Source catalog persistence beyond manifest-local source records.
- Archive, package, local, and generated source materialization.
- File watching and live workspace status streams.
- Branch and merge selection beyond v0 fast-forward and exact-commit paths.
- Alternate Git storage backends such as local mirrors or bare worktree stores.
- Remote capability enforcement and credential storage.
- Persistent `.gws/operations/<operation-id>.jsonl` event logs.
