use std::path::Path;

use crate::git::GitBackend;
use crate::model::ModelResult;
use crate::workspace::WORKSPACE_DIR;

pub(crate) fn stage_workspace_git_metadata<B: GitBackend>(
    backend: &B,
    root: &Path,
) -> ModelResult<()> {
    let mut pathspecs = vec![WORKSPACE_DIR];
    if root.join(".gitignore").is_file() {
        pathspecs.push(".gitignore");
    }
    backend.stage_paths(root, &pathspecs).map(|_| ())
}
