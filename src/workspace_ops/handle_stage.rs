use std::path::Path;

use crate::artifact;
use crate::git::GitBackend;
use crate::model::{ErrorCode, ModelError, ModelResult};
use crate::operation::OperationRequest;

use super::*;

/// Stage pathspecs across the repos that own them — the multi-repo `git add` verb
/// (GWZAddPlan). Pathspecs are resolved cwd-relative, routed to the innermost owning repo
/// (a member, or the workspace root) by [`resolve_stage_targets`], and staged there via
/// `stage_paths`. Local only: no lock mutation, no network. A targeted member must be
/// materialized.
pub fn handle_stage<B>(
    backend: &B,
    start: &Path,
    request: crate::StageRequest,
    operation_id: impl Into<String>,
) -> ModelResult<crate::StageResponse>
where
    B: GitBackend,
{
    let context = OperationRequest::Stage(request.clone()).context(operation_id.into())?;
    let root = resolve_workspace_root(start, request.meta.workspace.as_ref())?;
    let manifest = artifact::read_manifest(&root)?;
    assert_workspace_id(&manifest, request.meta.workspace.as_ref())?;

    // Every member path defines a repo boundary for routing.
    let member_paths: Vec<String> = manifest.members.iter().map(|m| m.path.clone()).collect();
    let all = request.all.unwrap_or(false);
    // A narrowing member selection (`--member` / `--member-path`) scopes `-A` to those
    // members only; bare `-A` (or `--all`) stages the root plus every member.
    let narrowed =
        request.meta.selection.as_ref().is_some_and(|selection| {
            !selection.member_ids.is_empty() || !selection.paths.is_empty()
        });

    let targets = if all && narrowed {
        let lock = artifact::read_lock(&root)?;
        let selected = resolve_locked_selection(&manifest, &lock, request.meta.selection.as_ref())?;
        selected
            .iter()
            .map(|member_id| {
                let path = manifest
                    .members
                    .iter()
                    .find(|member| &member.id == member_id)
                    .map(|member| member.path.clone())
                    .ok_or_else(|| {
                        ModelError::new(ErrorCode::MemberNotFound, "member not found")
                    })?;
                Ok(StageTarget {
                    member_path: Some(path),
                    pathspecs: vec![".".to_owned()],
                    explicit: true,
                })
            })
            .collect::<ModelResult<Vec<_>>>()?
    } else {
        resolve_stage_targets(
            &root,
            &member_paths,
            Path::new(&request.cwd),
            &request.pathspecs,
            all,
        )?
    };

    // Stage each target repo. An unmaterialized repo is an error if a pathspec named it
    // directly, but is skipped if it was only reached by `.` / `-A` fan-out.
    for target in &targets {
        let repo_root = match &target.member_path {
            Some(path) => root.join(path),
            None => root.clone(),
        };
        if !backend.is_repository(&repo_root)? {
            if target.explicit {
                return Err(ModelError::new(
                    ErrorCode::MemberNotFound,
                    format!(
                        "member '{}' is not materialized; cannot stage",
                        target.member_path.as_deref().unwrap_or("<root>")
                    ),
                ));
            }
            continue;
        }
        let pathspecs: Vec<&str> = target.pathspecs.iter().map(String::as_str).collect();
        backend.stage_paths(&repo_root, &pathspecs)?;
    }

    Ok(crate::StageResponse {
        response: response_envelope(context, crate::AggregateStatus::Ok, Vec::new()),
    })
}
