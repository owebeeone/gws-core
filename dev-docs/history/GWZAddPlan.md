# GWZ Add (multi-repo staging) Plan

Status: **implemented** (2026-06-23). §3 ratified (D1–D5 as recommended; D2 = match
`git add .`, D5 = `stage` protocol action). Phase 1 + Phase 3 done; Phase 2 covered except
two deferred follow-ups. Owner: Gianni.

Deferred (non-blocking follow-ups):
- **D3 selection scoping** for `-A` (`--only <member>`) — not wired; `-A` stages all repos.
- **Unmaterialized members on fan-out:** `gwz add .` / `-A` currently errors if any member
  is not materialized (strict). Could skip-on-fan-out instead (error only for *explicit*
  member targets).

`GWZDesign.md` stays authoritative for the overall design; this is the active
checkpoint for the `gwz add` verb (per the AGENTS.md scope rule). Sibling plan to the
`gwz commit` verb.

## 1. Goal & shape

Make `gwz add <pathspec>…` the **multi-repo `git add`**: stage paths in whichever repo
owns them, exactly as `git add` would if run inside that repo. This is the staging half
of the git-native multi-repo flow and pairs with `gwz commit`:

```
gwz add src/foo.rs gwz-cli/src/bar.rs
gwz commit -m "…"
```

The name is now free: "register an existing repo" moved to `gwz repo add`, so the
git-natural meaning of `gwz add` (stage files) is available.

Relationship to what exists:
- **Complements `gwz commit -a`.** `commit -a` stages *tracked* modifications inline;
  `gwz add` stages explicit pathspecs including **new/untracked** files and selective
  subsets — the cases `-a` can't express.
- **Reuses the `stage_paths` backend primitive** (`src/git/gitbackend.rs`) — no new git
  primitive. Its contract is already full `git add` semantics: add new/modified, remove
  deleted, honor `.gitignore`, self-verify. We call it once per owning repo.
- **Local only.** No network, no lock mutation — `gwz add` only touches member/root
  indexes. (Unlike `commit`, it does not re-lock or write `gwz.conf`.)

## 2. The crux — pathspec → repo routing

The workspace is a set of **nested git repos**: the root workspace repo at `<root>`
(tracks `gwz.conf`) plus each member repo at `<root>/<member.path>`. Members are hidden
from the root via `.git/info/exclude`, so every path is tracked by **exactly one** repo —
the innermost repo whose worktree contains it. Routing follows that:

1. **Normalize** each raw pathspec to an absolute path relative to **cwd** (git treats
   pathspecs as cwd-relative, not workspace-root-relative).
2. **Find the owning repo:** the innermost member whose `<root>/<member.path>` is a path
   prefix; if none, it belongs to the **root** repo (e.g. `gwz.conf/gwz.yml`). A path
   outside `<root>` → error.
3. **Re-root** the pathspec to be repo-relative (strip the member/root prefix).
4. **Group** repo-relative pathspecs by owning repo.
5. For each repo, call `stage_paths(repo_root, &repo_relative_pathspecs)`.
6. **Aggregate** the per-repo `GitStageResult`s into the response (which repos staged,
   counts).

This is a **pure function** — `(root, materialized members, cwd, pathspecs, all) → Vec<{repo_root, repo_relative_pathspecs}>` — and is the heart of the feature. Isolate it and test it hard (it is the one piece with real design surface; the rest is plumbing that mirrors `handle_commit`).

Special forms:
- **`.` / cwd-relative globs:** stage everything under cwd. cwd inside member X → stages
  in X only; cwd at the workspace root → spans members + root (see **D2**).
- **`-A` / `--all`:** workspace-wide stage-all across every repo (root + materialized
  members), independent of cwd — i.e. `stage_paths(repo, &["."])` per repo (its contract
  records deletions too).
- A target member must be **materialized** (a real repo on disk); targeting an
  unmaterialized member → error (mirror `handle_commit`'s pre-flight validation).

## 3. Decisions to confirm (ratify before Phase 2)

| ID | Decision | Recommendation |
|----|----------|----------------|
| D1 | Is the **root** workspace repo a valid stage target (so `gwz add gwz.conf/gwz.yml` works)? | **Yes** — the innermost-repo model includes root; root-level paths route to the root repo. |
| D2 | `gwz add .` at the **workspace root** — stage across *all* members+root, or restrict (root-only / require cwd-in-member)? | **Stage across all** repos the pathspec covers (matches `git add .` recursion). Powerful — document it. (Alt: error at root, require `-A`.) |
| D3 | `-A/--all` scope + optional member selection? | `-A` = all repos; allow `--only <member>`-style **selection** (reuse `resolve_locked_selection`) to scope it. |
| D4 | Pathspec **outside the workspace**, or in an **unmaterialized** member? | **Hard error** with a clear message — never silently skip. |
| D5 | **Protocol action** vs CLI-only? | New `stage` action — symmetry with `commit`, and a UI/daemon can stage. (Name it `stage`, not `add`, to avoid confusion with `add_existing_repo`.) |

## 4. Phased plan

Each phase is a shippable milestone; steps are single goals, aspirational < 500 LOC.
Foundational-first; steps marked **∥** can be picked up independently.

### Phase 1 — Working `gwz add` (MVP milestone)
A usable `gwz add <pathspec>…` that stages in the correct owning repo for the common
cases (explicit cwd-relative pathspecs; one or more members and/or root).

- **Step 1.1 ∥ — Protocol: `stage` action** (gwz-core). Add `stage` to `ActionKind`
  (next free ordinal — `commit` is the most recent verb action), `StageRequest { meta,
  pathspecs: List<STR>, all: BOOL? }`, `StageResponse` (per-repo staged summary), and the
  service method, in `protocol/gwz.taut.py`. Regenerate `src/protocol/generated.rs` +
  corpus; `corpus --check` green. *Mechanical; independent of routing.*
- **Step 1.2 ∥ — Pathspec→repo router** (gwz-core). A pure module (e.g.
  `workspace_ops/stage_routing.rs`) implementing §2 with a small error type. Table-driven
  unit tests: cwd at root, cwd in a member, cross-member set, root-level path,
  outside-workspace error, unmaterialized-member error. *The design crux; independent of
  the protocol.*
- **Step 1.3 — `handle_stage` handler** (gwz-core). Mirror `handle_commit`: resolve
  root/manifest/lock, validate targeted members materialized, call the router, fan out
  `stage_paths` per repo, aggregate into `StageResponse`; add the `stage` arm to the
  push-event / operation context. Handler tests via the tracking + Git2 backends.
  *Depends on 1.1 + 1.2.*
- **Step 1.4 ∥ — `gwz add` CLI verb** (gwz-cli). New top-level `Add(AddArgs)`: positional
  `pathspecs: Vec<String>`, `-A/--all`; `AddArgs::request → CliRequest::Stage(StageRequest)`;
  render the response; author **fresh** `add_long`/`add_after` help for the *staging*
  semantics (the old ones now live under `repo add`). *Depends only on 1.1's request type;
  buildable alongside 1.3.*

### Phase 2 — Full `git add` parity (semantics milestone)
- **Step 2.1 — Edge-case semantics.** Finalize D2 (`.`-at-root across repos), `-A`, and
  selection scoping (D3); confirm deletion handling via `stage_paths`; precise errors for
  D4. Unit + handler tests for each.

### Phase 3 — Docs & integration hardening (release-ready milestone)
- **Step 3.1 — Integration tests** (gwz-cli). End-to-end: `gwz add <file>` stages in the
  owning member; cross-member; `gwz add .`; then `gwz commit` over the result.
- **Step 3.2 — Docs.** Add `gwz add` to README "Current Commands", the `GWZDesign`
  command mapping, and (re)write the verb help. Full `cargo test` + `clippy` +
  `corpus --check` green.

## 5. Anchors in existing code
- `GitBackend::stage_paths` (`src/git/gitbackend.rs`) — the reused primitive.
- `handle_commit` (`src/workspace_ops/handle_commit.rs`) — the handler to mirror
  (selection, materialization pre-flight, per-member fan-out, response envelope).
- The freed CLI verb slot in `gwz-cli` (`globalargs.rs` / `clirequest.rs`) — `gwz add` is
  no longer wired to anything after the `repo add` rename.

## 6. Out of scope (later)
- Interactive / patch staging (`git add -p`).
- Unstaging (`gwz restore --staged` / `reset`) — a separate verb.
- Writing *member* paths into the root index — members are excluded; member paths always
  route to the member repo, never the root.
