use std::fs;
use std::path::Path;

use crate::git::{Git2Backend, GitBackend};

use super::*;

// P1.3: the `gwz add` handler routes pathspecs to their owning repos and stages there.

fn stage_request(cwd: &Path, pathspecs: &[&str], all: bool) -> crate::StageRequest {
    crate::StageRequest {
        meta: request_meta(),
        cwd: cwd.to_string_lossy().into_owned(),
        pathspecs: pathspecs.iter().map(|s| (*s).to_owned()).collect(),
        all: all.then_some(true),
    }
}

fn staged(backend: &Git2Backend, repo: &Path, path: &str) -> bool {
    backend
        .status(repo)
        .unwrap()
        .files
        .iter()
        .any(|file| file.path == path && file.index_status == "A")
}

#[test]
fn stages_pathspec_into_owning_member() {
    let temp = TempDir::new("stage-member");
    let backend = Git2Backend::new();
    let _fixture = init_one_member_workspace(temp.path(), &backend, "stage-member-source");
    let member_root = temp.path().join("remote");
    fs::write(member_root.join("new.txt"), "x\n").unwrap();
    assert_eq!(backend.status(&member_root).unwrap().staged, 0);

    let response = handle_stage(
        &backend,
        temp.path(),
        stage_request(temp.path(), &["remote/new.txt"], false),
        "op_stage",
    )
    .unwrap();

    assert_eq!(
        response.response.meta.aggregate_status,
        crate::AggregateStatus::Ok
    );
    assert!(staged(&backend, &member_root, "new.txt"), "new.txt staged in the member");
}

#[test]
fn stages_root_level_path_in_root_repo() {
    let temp = TempDir::new("stage-root");
    let backend = Git2Backend::new();
    let _fixture = init_one_member_workspace(temp.path(), &backend, "stage-root-source");
    fs::write(temp.path().join("root.txt"), "y\n").unwrap();

    handle_stage(
        &backend,
        temp.path(),
        stage_request(temp.path(), &["root.txt"], false),
        "op_stage",
    )
    .unwrap();
    assert!(staged(&backend, temp.path(), "root.txt"), "root.txt staged in the root repo");
}

#[test]
fn dot_at_root_stages_member_and_root() {
    let temp = TempDir::new("stage-dot");
    let backend = Git2Backend::new();
    let _fixture = init_one_member_workspace(temp.path(), &backend, "stage-dot-source");
    let member_root = temp.path().join("remote");
    fs::write(member_root.join("a.txt"), "x\n").unwrap();
    fs::write(temp.path().join("root.txt"), "y\n").unwrap();

    handle_stage(&backend, temp.path(), stage_request(temp.path(), &["."], false), "op_stage").unwrap();
    assert!(staged(&backend, &member_root, "a.txt"), "member file staged");
    assert!(staged(&backend, temp.path(), "root.txt"), "root file staged");
}

#[test]
fn all_flag_stages_member_and_root() {
    let temp = TempDir::new("stage-all");
    let backend = Git2Backend::new();
    let _fixture = init_one_member_workspace(temp.path(), &backend, "stage-all-source");
    let member_root = temp.path().join("remote");
    fs::write(member_root.join("a.txt"), "x\n").unwrap();
    fs::write(temp.path().join("root.txt"), "y\n").unwrap();

    handle_stage(&backend, temp.path(), stage_request(temp.path(), &[], true), "op_stage").unwrap();
    assert!(staged(&backend, &member_root, "a.txt"), "member file staged via -A");
    assert!(staged(&backend, temp.path(), "root.txt"), "root file staged via -A");
}

#[test]
fn pathspec_outside_workspace_errors() {
    let temp = TempDir::new("stage-escape");
    let backend = Git2Backend::new();
    let _fixture = init_one_member_workspace(temp.path(), &backend, "stage-escape-source");

    let err = handle_stage(
        &backend,
        temp.path(),
        stage_request(temp.path(), &["../escape.txt"], false),
        "op_stage",
    )
    .unwrap_err();
    assert_eq!(err.code, crate::model::ErrorCode::PathEscape);
}
