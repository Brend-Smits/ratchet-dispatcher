use std::fs;

use log::{debug, error};

pub fn cleanup_clone_dir(local_path: &str) {
    if fs::remove_dir_all(local_path).is_ok() {
        debug!("Cleaned up temporary directory: {}", local_path);
    } else {
        error!("Failed to clean up temporary directory: {}", local_path);
    }
}

// If the user has a custom PR body, we should read the file and use that as the PR body
// Otherwise, we should use a default PR body
pub fn get_pr_body_from_file(pr_body_path: &Option<String>) -> String {
    match pr_body_path {
        Some(path) => {
            fs::read_to_string(path).unwrap()
        }
        None => {
            String::from(
                "This automatically generated pull request upgrades the workflows using ratchet. It pins the versions of the actions used in the workflows to prevent bad actors from overwriting tags/versions. Please review the changes and merge if everything looks good.",
            )
        }
    }
}
