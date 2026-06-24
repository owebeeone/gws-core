use std::path::Path;

use crate::artifact;
use crate::model::ModelResult;
use crate::operation::OperationRequest;

use super::*;

/// `gwz ls` — list the workspace's members (`id`, `path`, `abspath`, `materialized`). A read-only
/// op: manifest + lock only, **no git**. Selection rides in `meta.selection` (the global
/// `--member`/`-A`). `include_unmaterialized` surfaces configured-but-uncloned members; by default
/// only materialized members are listed (so `cd $path` can't fail). The filter is uniform — an
/// explicitly-selected member that isn't materialized is simply omitted unless `include_unmaterialized`
/// is set (a non-existent member still errors via selection resolution).
pub fn handle_ls(
    start: &Path,
    request: crate::LsRequest,
    operation_id: impl Into<String>,
) -> ModelResult<crate::LsResponse> {
    let context = OperationRequest::Ls(request.clone()).context(operation_id.into())?;
    let root = resolve_workspace_root(start, request.meta.workspace.as_ref())?;
    let manifest = artifact::read_manifest(&root)?;
    assert_workspace_id(&manifest, request.meta.workspace.as_ref())?;

    // Read the lock if present; its absence just means nothing is materialized yet.
    let lock = if root.join(artifact::LOCK_PATH).exists() {
        Some(artifact::read_lock(&root)?)
    } else {
        None
    };
    let include_unmaterialized = request.include_unmaterialized.unwrap_or(false);

    // Manifest-tolerant selection (no lock-presence requirement, unlike resolve_locked_selection).
    let selected = crate::status::resolve_selection(&manifest, request.meta.selection.as_ref())?;
    let members = selected
        .into_iter()
        .filter_map(|member| {
            let materialized = lock
                .as_ref()
                .and_then(|lock| lock.members.get(&member.id))
                .and_then(|entry| entry.materialized)
                == Some(true);
            (materialized || include_unmaterialized).then(|| crate::MemberEntry {
                id: member.id.clone(),
                path: member.path.clone(),
                abspath: root.join(&member.path).to_string_lossy().into_owned(),
                materialized,
            })
        })
        .collect();

    Ok(crate::LsResponse {
        response: response_envelope(context, crate::AggregateStatus::Ok, Vec::new()),
        members: Some(members),
    })
}
