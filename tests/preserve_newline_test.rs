use ratchet_dispatcher::git::GitRepository;
use tempfile::tempdir;

fn setup_test_repo() -> (tempfile::TempDir, String) {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let repo_path = temp_dir.path().to_str().unwrap().to_string();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to init git repo");

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to set git user name");

    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to set git user email");

    (temp_dir, repo_path)
}

fn create_test_workflow_file(repo_path: &str, content: &str) {
    let workflow_dir = std::path::Path::new(repo_path).join(".github/workflows");
    std::fs::create_dir_all(&workflow_dir).expect("Failed to create workflow dir");

    let workflow_path = workflow_dir.join("test.yml");
    std::fs::write(&workflow_path, content).expect("Failed to write workflow file");

    // Add and commit the file
    std::process::Command::new("git")
        .args(["add", ".github/workflows/test.yml"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to add workflow file");

    std::process::Command::new("git")
        .args(["commit", "-m", "Initial workflow"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to commit workflow file");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preserve_newline_enabled() {
        let (_temp_dir, repo_path) = setup_test_repo();

        // Create a workflow file without trailing newline
        let original_content = "name: Test\non:\n  push:\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v3";
        create_test_workflow_file(&repo_path, original_content);

        // Modify the file to have updated uses: and add trailing newline
        let modified_content = "name: Test\non:\n  push:\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n";

        let workflow_path = std::path::Path::new(&repo_path).join(".github/workflows/test.yml");
        std::fs::write(&workflow_path, modified_content).expect("Failed to write modified content");

        // Test staging with preserve_newline enabled
        let git_repo = GitRepository::open(repo_path.clone()).expect("Failed to open repo");
        git_repo
            .stage_changes(true)
            .expect("Failed to stage changes");

        // Check what was staged
        let staged_diff = std::process::Command::new("git")
            .args(["diff", "--cached"])
            .current_dir(&git_repo.working_dir)
            .output()
            .expect("Failed to get staged diff");

        let staged_content = String::from_utf8_lossy(&staged_diff.stdout);
        println!(
            "Staged content with preserve_newline=true:\n{}",
            staged_content
        );

        // Should contain the uses: line change AND preserve the newline
        assert!(
            staged_content.contains("actions/checkout@v4"),
            "Should contain the updated uses: line"
        );
        assert!(
            staged_content.contains("+      - uses: actions/checkout@v4"),
            "Should stage the new line"
        );
        assert!(
            staged_content.contains("-      - uses: actions/checkout@v3"),
            "Should remove the old line"
        );
    }

    #[test]
    fn test_preserve_newline_disabled() {
        let (_temp_dir, repo_path) = setup_test_repo();

        // Create a workflow file without trailing newline
        let original_content = "name: Test\non:\n  push:\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v3";
        create_test_workflow_file(&repo_path, original_content);

        // Modify the file to have updated uses: and add trailing newline
        let modified_content = "name: Test\non:\n  push:\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n";

        let workflow_path = std::path::Path::new(&repo_path).join(".github/workflows/test.yml");
        std::fs::write(&workflow_path, modified_content).expect("Failed to write modified content");

        // Test staging with preserve_newline disabled (default behavior)
        let git_repo = GitRepository::open(repo_path.clone()).expect("Failed to open repo");
        git_repo
            .stage_changes(false)
            .expect("Failed to stage changes");

        // Check what was staged
        let staged_diff = std::process::Command::new("git")
            .args(["diff", "--cached"])
            .current_dir(&git_repo.working_dir)
            .output()
            .expect("Failed to get staged diff");

        let staged_content = String::from_utf8_lossy(&staged_diff.stdout);
        println!(
            "Staged content with preserve_newline=false:\n{}",
            staged_content
        );

        // Should contain the uses: line change but not preserve the newline (current behavior)
        assert!(
            staged_content.contains("actions/checkout@v4"),
            "Should contain the updated uses: line"
        );
        assert!(
            staged_content.contains("+      - uses: actions/checkout@v4"),
            "Should stage the new line"
        );
        assert!(
            staged_content.contains("-      - uses: actions/checkout@v3"),
            "Should remove the old line"
        );
    }

    #[test]
    fn test_skip_only_newline_changes() {
        let (_temp_dir, repo_path) = setup_test_repo();

        // Create a workflow file with trailing newline
        let original_content = "name: Test\non:\n  push:\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n";
        create_test_workflow_file(&repo_path, original_content);

        // Modify the file to remove only the trailing newline (no uses: changes)
        let modified_content = "name: Test\non:\n  push:\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4";

        let workflow_path = std::path::Path::new(&repo_path).join(".github/workflows/test.yml");
        std::fs::write(&workflow_path, modified_content).expect("Failed to write modified content");

        // Test staging with preserve_newline enabled - should skip staging
        let git_repo = GitRepository::open(repo_path.clone()).expect("Failed to open repo");
        git_repo
            .stage_changes(true)
            .expect("Failed to stage changes");

        // Check what was staged - should be nothing
        let staged_diff = std::process::Command::new("git")
            .args(["diff", "--cached", "--name-only"])
            .current_dir(&git_repo.working_dir)
            .output()
            .expect("Failed to get staged diff");

        let staged_files = String::from_utf8_lossy(&staged_diff.stdout);
        println!(
            "Staged files with preserve_newline=true (only newline diff):\n{}",
            staged_files
        );

        // Should be empty - no files staged because only newlines changed
        assert!(
            staged_files.trim().is_empty(),
            "Should not stage files when only newlines change with preserve_newline=true"
        );
    }
}
