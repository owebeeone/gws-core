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
    let targets = resolve_stage_targets(
        &root,
        &member_paths,
        Path::new(&request.cwd),
        &request.pathspecs,
        request.all.unwrap_or(false),
    )?;

    for target in &targets {
        let repo_root = match &target.member_path {
            Some(path) => root.join(path),
            None => root.clone(),
        };
        if !backend.is_repository(&repo_root)? {
            return Err(ModelError::new(
                ErrorCode::MemberNotFound,
                format!(
                    "member '{}' is not materialized; cannot stage",
                    target.member_path.as_deref().unwrap_or("<root>")
                ),
            ));
        }
        let pathspecs: Vec<&str> = target.pathspecs.iter().map(String::as_str).collect();
        backend.stage_paths(&repo_root, &pathspecs)?;
    }

    Ok(crate::StageResponse {
        response: response_envelope(context, crate::AggregateStatus::Ok, Vec::new()),
    })
}
