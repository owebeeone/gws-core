use std::fs;

use super::*;

// §5.1 / G3 / G4: `ensure_gitignore_tmp` only appends the static `/gwz.conf/.tmp/`
// line when absent — it never strips or rewrites existing content.

#[test]
fn ensure_gitignore_tmp_appends_when_absent() {
    let temp = TempDir::new("gitignore-tmp-absent");
    ensure_gitignore_tmp(temp.path()).unwrap();
    let ignore = fs::read_to_string(temp.path().join(".gitignore")).unwrap();
    assert!(ignore.contains("/gwz.conf/.tmp/"));
}

#[test]
fn ensure_gitignore_tmp_is_idempotent() {
    let temp = TempDir::new("gitignore-tmp-idem");
    ensure_gitignore_tmp(temp.path()).unwrap();
    let once = fs::read_to_string(temp.path().join(".gitignore")).unwrap();
    ensure_gitignore_tmp(temp.path()).unwrap();
    let twice = fs::read_to_string(temp.path().join(".gitignore")).unwrap();
    assert_eq!(once, twice, "second call must not change the file");
    assert_eq!(
        twice.matches("/gwz.conf/.tmp/").count(),
        1,
        "no duplicate tmp entry"
    );
}

#[test]
fn ensure_gitignore_tmp_preserves_user_lines_and_appends() {
    let temp = TempDir::new("gitignore-tmp-user");
    fs::write(temp.path().join(".gitignore"), "/build/\n").unwrap();
    ensure_gitignore_tmp(temp.path()).unwrap();
    let after = fs::read_to_string(temp.path().join(".gitignore")).unwrap();
    assert!(after.starts_with("/build/\n"), "user line preserved: {after:?}");
    assert!(after.contains("/gwz.conf/.tmp/"));
}

#[test]
fn ensure_gitignore_tmp_leaves_legacy_block_untouched() {
    // Migration (G3): a pre-existing legacy managed block is left exactly as-is. The
    // old writer always included the tmp line, so this is a pure no-op — never stripped.
    let temp = TempDir::new("gitignore-tmp-legacy");
    let legacy = "# BEGIN GWZ managed member repositories\n/gwz.conf/.tmp/\n/remote/\n# END GWZ managed member repositories\n";
    fs::write(temp.path().join(".gitignore"), legacy).unwrap();
    ensure_gitignore_tmp(temp.path()).unwrap();
    let after = fs::read_to_string(temp.path().join(".gitignore")).unwrap();
    assert_eq!(after, legacy, "legacy block left exactly as-is, never stripped");
}
