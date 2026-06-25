use crate::artifact::{read_lock, read_snapshot};
use crate::git::{Git2Backend, GitBackend};
use crate::model::ErrorCode;
use crate::operation::NullSink;

use super::*;

// GwzStashBranchPlan B3: core materialize branch and snapshot branch behavior.

fn branch_materialize_request(branch: &str) -> crate::MaterializeRequest {
    materialize_named_request(crate::MaterializeTargetKind::Branch, branch)
}

fn snapshot_request(
    snapshot_id: &str,
    source: Option<crate::SnapshotSource>,
) -> crate::SnapshotRequest {
    crate::SnapshotRequest {
        meta: request_meta_with_workspace(),
        snapshot_id: snapshot_id.to_owned(),
        source,
    }
}

fn branch_source(branch: &str) -> crate::SnapshotSource {
    crate::SnapshotSource {
        kind: crate::SnapshotSourceKind::Branch,
        branch: Some(branch.to_owned()),
    }
}

fn create_branch_at_head(repo_path: &std::path::Path, branch: &str) -> String {
    let repo = git2::Repository::open(repo_path).unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    repo.branch(branch, &head, false).unwrap();
    head.id().to_string()
}

fn checkout_branch(repo_path: &std::path::Path, branch: &str) {
    Git2Backend::new().switch_branch(repo_path, branch).unwrap();
}

fn init_two_member_workspace(
    temp: &std::path::Path,
    backend: &Git2Backend,
) -> (RemoteFixture, RemoteFixture) {
    let fa = RemoteFixture::new("b3-two-app");
    fa.commit_and_push("README.md", "a", "init a", backend);
    let fb = RemoteFixture::new("b3-two-lib");
    fb.commit_and_push("README.md", "b", "init b", backend);
    handle_init_from_sources(
        backend,
        temp,
        crate::InitFromSourcesRequest {
            meta: request_meta(),
            workspace_root: temp.to_string_lossy().into_owned(),
            sources: vec![
                crate::SourceUrl {
                    url: fa.remote_url().to_owned(),
                    path: Some("app".to_owned()),
                    remote_name: None,
                    branch: None,
                },
                crate::SourceUrl {
                    url: fb.remote_url().to_owned(),
                    path: Some("lib".to_owned()),
                    remote_name: None,
                    branch: None,
                },
            ],
            target: None,
            workspace_id: Some("ws_ops".to_owned()),
        },
        "op_init",
        &NullSink,
    )
    .unwrap();
    (fa, fb)
}

#[test]
fn materialize_branch_switches_and_rewrites_lock_from_observed_state() {
    let temp = TempDir::new("mat-branch-lock");
    let backend = Git2Backend::new();
    let _fixture = init_one_member_workspace(temp.path(), &backend, "mat-branch-lock-source");
    let member_root = temp.path().join("remote");
    let feature_commit = create_branch_at_head(&member_root, "feature");

    handle_materialize(
        &backend,
        temp.path(),
        branch_materialize_request("feature"),
        "op_branch",
        &NullSink,
    )
    .unwrap();

    let head = backend.head(&member_root).unwrap();
    assert_eq!(head.branch.as_deref(), Some("feature"));
    assert_eq!(head.commit.as_deref(), Some(feature_commit.as_str()));
    let locked = &read_lock(temp.path()).unwrap().members["mem_remote"];
    assert_eq!(locked.branch.as_deref(), Some("feature"));
    assert_eq!(locked.commit.as_deref(), Some(feature_commit.as_str()));
    assert_eq!(locked.detached, Some(false));
    assert_eq!(locked.dirty, Some(false));
}

#[test]
fn materialize_branch_does_not_create_missing_branches() {
    let temp = TempDir::new("mat-branch-missing");
    let backend = Git2Backend::new();
    let _fixture = init_one_member_workspace(temp.path(), &backend, "mat-branch-missing-source");
    let member_root = temp.path().join("remote");
    let before = backend.head(&member_root).unwrap();

    let err = handle_materialize(
        &backend,
        temp.path(),
        branch_materialize_request("missing"),
        "op_branch",
        &NullSink,
    )
    .unwrap_err();

    assert_eq!(err.code, ErrorCode::GitCommandFailed);
    assert_eq!(
        backend
            .read_ref(&member_root, "refs/heads/missing")
            .unwrap(),
        None
    );
    assert_eq!(backend.head(&member_root).unwrap(), before);
}

#[test]
fn materialize_branch_rejects_dirty_members_before_any_mutation() {
    let temp = TempDir::new("mat-branch-dirty");
    let backend = Git2Backend::new();
    let _fixture = init_one_member_workspace(temp.path(), &backend, "mat-branch-dirty-source");
    let member_root = temp.path().join("remote");
    create_branch_at_head(&member_root, "feature");
    std::fs::write(member_root.join("dirty.txt"), "dirty\n").unwrap();
    let before = backend.head(&member_root).unwrap();

    let err = handle_materialize(
        &backend,
        temp.path(),
        branch_materialize_request("feature"),
        "op_branch",
        &NullSink,
    )
    .unwrap_err();

    assert_eq!(err.code, ErrorCode::DirtyMember);
    assert_eq!(backend.head(&member_root).unwrap(), before);
}

#[test]
fn snapshot_current_branch_rejects_detached_head_and_mixed_branches() {
    let temp = TempDir::new("snap-current-branch");
    let backend = Git2Backend::new();
    let (_fa, _fb) = init_two_member_workspace(temp.path(), &backend);
    let app = temp.path().join("app");
    let lib = temp.path().join("lib");
    create_branch_at_head(&app, "feature");
    checkout_branch(&app, "feature");

    let mixed = handle_snapshot(
        &backend,
        temp.path(),
        snapshot_request(
            "mixed",
            Some(crate::SnapshotSource {
                kind: crate::SnapshotSourceKind::Current,
                branch: None,
            }),
        ),
        "op_snapshot",
    )
    .unwrap_err();
    assert_eq!(mixed.code, ErrorCode::BranchMixed);

    let lib_head = backend.head(&lib).unwrap().commit.unwrap();
    backend.checkout_commit(&lib, &lib_head).unwrap();
    let detached = handle_snapshot(
        &backend,
        temp.path(),
        snapshot_request(
            "detached",
            Some(crate::SnapshotSource {
                kind: crate::SnapshotSourceKind::Current,
                branch: None,
            }),
        ),
        "op_snapshot",
    )
    .unwrap_err();
    assert_eq!(detached.code, ErrorCode::BranchDetachedHead);
}

#[test]
fn snapshot_named_branch_does_not_change_worktrees_and_ignores_unrelated_dirtiness() {
    let temp = TempDir::new("snap-named-branch");
    let backend = Git2Backend::new();
    let (_fa, _fb) = init_two_member_workspace(temp.path(), &backend);
    let app = temp.path().join("app");
    let lib = temp.path().join("lib");
    let app_feature = create_branch_at_head(&app, "feature");
    let lib_feature = create_branch_at_head(&lib, "feature");
    std::fs::write(app.join("dirty.txt"), "dirty\n").unwrap();
    let app_head_before = backend.head(&app).unwrap();
    let lib_head_before = backend.head(&lib).unwrap();

    handle_snapshot(
        &backend,
        temp.path(),
        snapshot_request("feature_snap", Some(branch_source("feature"))),
        "op_snapshot",
    )
    .unwrap();

    assert_eq!(backend.head(&app).unwrap(), app_head_before);
    assert_eq!(backend.head(&lib).unwrap(), lib_head_before);
    let snapshot = read_snapshot(temp.path(), "feature_snap").unwrap();
    assert_eq!(
        snapshot.members["mem_app"].commit.as_deref(),
        Some(app_feature.as_str())
    );
    assert_eq!(
        snapshot.members["mem_lib"].commit.as_deref(),
        Some(lib_feature.as_str())
    );
    for state in snapshot.members.values() {
        assert_eq!(state.branch.as_deref(), Some("feature"));
        assert_eq!(state.detached, Some(false));
        assert_eq!(state.dirty, Some(false));
    }
}
