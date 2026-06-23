use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use crate::model::{ErrorCode, ModelError, ModelResult};

/// A repo to stage into plus its repo-relative pathspecs. `member_path == None` is the
/// workspace root repo; `Some(path)` is the member at `root/<path>`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StageTarget {
    pub member_path: Option<String>,
    pub pathspecs: Vec<String>,
}

/// Route raw `gwz add` pathspecs to the repos that own them (GWZAddPlan §2). The workspace
/// is nested repos (root + members); each path is owned by the innermost repo containing
/// it. Pathspecs are resolved cwd-relative (like `git add`), then mapped to that repo. A
/// directory pathspec at/above member boundaries fans out into each contained member, so
/// `gwz add .` at the root spans every repo (D2). `all` ignores pathspecs and targets the
/// root repo plus every member. Pure — no filesystem access (works for not-yet-existing /
/// deleted paths). Targets are returned root-first then by member path; pathspecs are
/// sorted and de-duplicated.
pub(crate) fn resolve_stage_targets(
    root: &Path,
    member_paths: &[String],
    cwd: &Path,
    pathspecs: &[String],
    all: bool,
) -> ModelResult<Vec<StageTarget>> {
    let mut groups: BTreeMap<Option<String>, BTreeSet<String>> = BTreeMap::new();

    if all {
        groups.entry(None).or_default().insert(".".to_owned());
        for member in member_paths {
            groups
                .entry(Some(member.clone()))
                .or_default()
                .insert(".".to_owned());
        }
        return Ok(into_targets(groups));
    }

    if pathspecs.is_empty() {
        return Err(ModelError::new(
            ErrorCode::InvalidRequest,
            "nothing specified to stage; pass pathspecs or --all",
        ));
    }

    for spec in pathspecs {
        let abs = lexical_normalize(&join_cwd(cwd, spec));
        let rel = abs.strip_prefix(root).map_err(|_| {
            ModelError::new(
                ErrorCode::PathEscape,
                format!("pathspec '{spec}' is outside the workspace"),
            )
        })?;

        // Innermost member whose path is a component-wise prefix of `rel`.
        let owner = member_paths
            .iter()
            .filter(|member| rel.starts_with(member.as_str()))
            .max_by_key(|member| Path::new(member.as_str()).components().count())
            .cloned();

        if let Some(member) = owner {
            let inner = rel.strip_prefix(&member).unwrap_or(rel);
            let spec = pathspec_str(inner);
            groups.entry(Some(member)).or_default().insert(spec);
        } else {
            // Root-territory path: stage it in the root repo, and fan out into every
            // member contained within this pathspec (D2 — members are excluded from the
            // root, so a root-side `.` would never reach them).
            groups.entry(None).or_default().insert(pathspec_str(rel));
            for member in member_paths {
                if rel.as_os_str().is_empty() || Path::new(member).starts_with(rel) {
                    groups
                        .entry(Some(member.clone()))
                        .or_default()
                        .insert(".".to_owned());
                }
            }
        }
    }

    Ok(into_targets(groups))
}

fn join_cwd(cwd: &Path, spec: &str) -> PathBuf {
    let path = Path::new(spec);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}

/// Lexically resolve `.` / `..` without touching the filesystem (unlike `normalize_path`,
/// which canonicalizes — wrong for not-yet-existing or deleted paths, and for symlinks).
fn lexical_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(value) => out.push(value),
            Component::RootDir | Component::Prefix(_) => out.push(component.as_os_str()),
        }
    }
    out
}

/// Repo-relative pathspec string: the repo root itself (empty) becomes ".", and path
/// separators are normalized to `/` for Git.
fn pathspec_str(rel: &Path) -> String {
    if rel.as_os_str().is_empty() {
        return ".".to_owned();
    }
    rel.to_string_lossy().replace('\\', "/")
}

fn into_targets(groups: BTreeMap<Option<String>, BTreeSet<String>>) -> Vec<StageTarget> {
    groups
        .into_iter()
        .map(|(member_path, pathspecs)| StageTarget {
            member_path,
            pathspecs: pathspecs.into_iter().collect(),
        })
        .collect()
}
