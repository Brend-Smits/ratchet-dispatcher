use ratchet_dispatcher::git::GitRepository;
use tempfile::tempdir;

fn setup_test_repo_with_commits() -> (tempfile::TempDir, String) {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let repo_path = temp_dir.path().to_str().unwrap().to_string();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init", "--initial-branch=main"])
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

    // Create a workflow file and commit it to main
    let workflow_dir = std::path::Path::new(&repo_path).join(".github/workflows");
    std::fs::create_dir_all(&workflow_dir).expect("Failed to create workflow dir");

    let workflow_path = workflow_dir.join("test.yml");
    let initial_content = "name: Test\non:\n  push:\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v3\n";
    std::fs::write(&workflow_path, initial_content).expect("Failed to write workflow file");

    // Add and commit to main
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to add files");

    std::process::Command::new("git")
        .args(["commit", "-m", "Initial commit on main"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to commit to main");

    // Create and switch to feature branch
    std::process::Command::new("git")
        .args(["checkout", "-b", "feature-branch"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to create feature branch");

    // Make some changes and commit to feature branch
    let modified_content = "name: Test\non:\n  push:\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - name: Some change\n        run: echo 'change'\n";
    std::fs::write(&workflow_path, modified_content).expect("Failed to write modified content");

    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to add modified files");

    std::process::Command::new("git")
        .args(["commit", "-m", "Modified workflow with extra changes"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to commit to feature branch");

    (temp_dir, repo_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overwrite_functionality() {
        let (_temp_dir, repo_path) = setup_test_repo_with_commits();

        // Check that we're on feature-branch and have commits ahead of main
        let branch_output = std::process::Command::new("git")
            .args(["branch", "--show-current"])
            .current_dir(&repo_path)
            .output()
            .expect("Failed to get current branch");
        let current_branch = String::from_utf8_lossy(&branch_output.stdout);
        assert_eq!(current_branch.trim(), "feature-branch");

        // Check commits ahead of main
        let ahead_output = std::process::Command::new("git")
            .args(["rev-list", "--count", "main..HEAD"])
            .current_dir(&repo_path)
            .output()
            .expect("Failed to count commits ahead");
        let commits_ahead = String::from_utf8_lossy(&ahead_output.stdout);
        println!(
            "Commits ahead of main before reset: {}",
            commits_ahead.trim()
        );
        assert!(commits_ahead.trim().parse::<i32>().unwrap() > 0);

        // Test reset_branch_to_base function
        let git_repo = GitRepository::open(repo_path.clone()).expect("Failed to open repo");

        // Set up remote origin (simulating a remote repo)
        std::process::Command::new("git")
            .args(["remote", "add", "origin", &repo_path])
            .current_dir(&repo_path)
            .output()
            .expect("Failed to add remote");

        // For testing, we'll mock the remote by creating a bare repo
        let bare_repo_path = format!("{}_bare", repo_path);
        std::process::Command::new("git")
            .args(["clone", "--bare", &repo_path, &bare_repo_path])
            .output()
            .expect("Failed to create bare repo");

        // Update origin to point to bare repo
        std::process::Command::new("git")
            .args(["remote", "set-url", "origin", &bare_repo_path])
            .current_dir(&repo_path)
            .output()
            .expect("Failed to set remote URL");

        // Push main to origin
        std::process::Command::new("git")
            .args(["push", "origin", "main"])
            .current_dir(&repo_path)
            .output()
            .expect("Failed to push main");

        // Switch back to feature branch
        std::process::Command::new("git")
            .args(["checkout", "feature-branch"])
            .current_dir(&repo_path)
            .output()
            .expect("Failed to checkout feature branch");

        // Now test the reset functionality
        let result = git_repo.reset_branch_to_base("main");
        assert!(result.is_ok(), "reset_branch_to_base should succeed");

        // Check that we're still on feature-branch but reset to main's state
        let branch_output = std::process::Command::new("git")
            .args(["branch", "--show-current"])
            .current_dir(&repo_path)
            .output()
            .expect("Failed to get current branch");
        let current_branch = String::from_utf8_lossy(&branch_output.stdout);
        assert_eq!(current_branch.trim(), "feature-branch");

        // Check that there are no commits ahead of main now
        let ahead_output = std::process::Command::new("git")
            .args(["rev-list", "--count", "main..HEAD"])
            .current_dir(&repo_path)
            .output()
            .expect("Failed to count commits ahead");
        let commits_ahead = String::from_utf8_lossy(&ahead_output.stdout);
        println!(
            "Commits ahead of main after reset: {}",
            commits_ahead.trim()
        );
        assert_eq!(commits_ahead.trim(), "0");

        // Check that the working directory has the original content from main
        let workflow_path = std::path::Path::new(&repo_path).join(".github/workflows/test.yml");
        let current_content =
            std::fs::read_to_string(&workflow_path).expect("Failed to read workflow file");

        // Should have the original content from main (v3, no extra changes)
        assert!(current_content.contains("actions/checkout@v3"));
        assert!(!current_content.contains("Some change"));
        assert!(!current_content.contains("echo 'change'"));
    }
}
