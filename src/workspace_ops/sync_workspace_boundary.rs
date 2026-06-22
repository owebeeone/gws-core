use std::fs;
use std::path::Path;

use crate::artifact::LockArtifact;
use crate::git::GitBackend;
use crate::model::ModelResult;
use crate::workspace::WORKSPACE_DIR;

use super::*;

/// Sync the git representation of the workspace to the lock (GWZGitlinkPlan §4.2):
/// ensure the static tmp-ignore line, project each materialized member as a gitlink in
/// the root index, and stage the workspace metadata. Goes through the passed backend so
/// a fake no-ops it in unit tests. Best-effort after the lock (G6) — the index is a
/// rebuildable projection of the lock, so a crash here is repaired by the next op.
pub(crate) fn sync_workspace_boundary<B: GitBackend>(
    backend: &B,
    root: &Path,
    lock: &LockArtifact,
) -> ModelResult<()> {
    ensure_gitignore_tmp(root)?;
    let desired = desired_gitlinks(lock);
    let refs: Vec<(&str, &str)> = desired
        .iter()
        .map(|(path, commit)| (path.as_str(), commit.as_str()))
        .collect();
    backend.sync_gitlinks(root, &refs)?;
    stage_workspace_git_metadata(backend, root)
}

/// The `(member path, commit oid)` pairs to project as gitlinks: every lock member that
/// is materialized with a recorded commit. Unmaterialized / carry-lock / unborn members
/// (no on-disk repo or no oid) get no gitlink — the reconcile drops any stale entry.
pub(crate) fn desired_gitlinks(lock: &LockArtifact) -> Vec<(String, String)> {
    lock.members
        .values()
        .filter(|member| member.materialized == Some(true))
        .filter_map(|member| {
            member
                .commit
                .as_ref()
                .map(|commit| (member.path.clone(), commit.clone()))
        })
        .collect()
}

/// Ensure `.gitignore` contains the static `/gwz.conf/.tmp/` line (G3/G4): append it if
/// absent, otherwise no-op. Never strips or rewrites existing content — old entries
/// (including a legacy managed block) are left exactly as they are.
pub(crate) fn ensure_gitignore_tmp(root: &Path) -> ModelResult<()> {
    let entry = format!("/{WORKSPACE_DIR}/.tmp/");
    let path = root.join(".gitignore");
    let existing = match fs::read_to_string(&path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(io_error(error)),
    };
    if existing.lines().any(|line| line.trim() == entry) {
        return Ok(());
    }
    let mut updated = existing;
    if !updated.is_empty() && !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(&entry);
    updated.push('\n');
    fs::write(&path, updated).map_err(io_error)
}
