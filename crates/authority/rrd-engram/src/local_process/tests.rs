use super::same_executable_file;

#[test]
fn executable_identity_accepts_hard_links_but_not_copies() {
    let temporary = tempfile::tempdir().unwrap();
    let expected = temporary.path().join("expected");
    let hard_link = temporary.path().join("hard-link");
    let copy = temporary.path().join("copy");
    std::fs::write(&expected, b"authenticated executable image").unwrap();
    std::fs::hard_link(&expected, &hard_link).unwrap();
    std::fs::copy(&expected, &copy).unwrap();

    assert!(same_executable_file(&expected, &expected));
    assert!(same_executable_file(&hard_link, &expected));
    assert!(!same_executable_file(&copy, &expected));
}
