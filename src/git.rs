use std::process::Command;

pub struct GitRepository {
    pub working_dir: String,
}

impl GitRepository {
    pub fn open(working_dir: String) -> Result<Self, String> {
        Ok(GitRepository { working_dir })
    }

    pub fn stage_changes(&self) -> Result<(), String> {
        // Get list of modified files with 'uses:' changes
        let modified_files = self.get_files_with_uses_changes()?;

        if modified_files.is_empty() {
            log::info!("No files with uses: changes found");
            return Ok(());
        }

        log::info!("Found {} files with uses: changes", modified_files.len());

        // Stage only the uses: lines from each file
        for file in modified_files {
            self.stage_uses_lines_only(&file)?;
        }

        Ok(())
    }

    fn stage_uses_lines_only(&self, file: &str) -> Result<(), String> {
        // Much simpler approach: create a temporary branch and cherry-pick only uses: changes

        // First, create a copy of the original file content from HEAD
        let original_output = Command::new("git")
            .args(["show", &format!("HEAD:{}", file)])
            .current_dir(&self.working_dir)
            .output()
            .map_err(|e| format!("Failed to get original file {}: {}", file, e))?;

        if !original_output.status.success() {
            return Err(format!(
                "Git show failed for {}: {}",
                file,
                String::from_utf8_lossy(&original_output.stderr)
            ));
        }

        let original_content = String::from_utf8_lossy(&original_output.stdout);

        // Get current file content
        let current_path = std::path::Path::new(&self.working_dir).join(file);
        let current_content = std::fs::read_to_string(&current_path)
            .map_err(|e| format!("Failed to read current file {}: {}", file, e))?;

        // Create a new version with only uses: line changes
        let uses_only_content =
            self.create_uses_only_version(&original_content, &current_content)?;

        // Temporarily overwrite the file with the uses-only version
        std::fs::write(&current_path, &uses_only_content)
            .map_err(|e| format!("Failed to write uses-only content: {}", e))?;

        // Stage the file
        let stage_output = Command::new("git")
            .args(["add", file])
            .current_dir(&self.working_dir)
            .output()
            .map_err(|e| format!("Failed to stage file: {}", e))?;

        if !stage_output.status.success() {
            // Restore original content before returning error
            std::fs::write(&current_path, current_content)
                .map_err(|e| format!("Failed to restore file after staging error: {}", e))?;
            return Err(format!(
                "Failed to stage file {}: {}",
                file,
                String::from_utf8_lossy(&stage_output.stderr)
            ));
        }

        // Restore the original current content to working directory
        std::fs::write(&current_path, current_content)
            .map_err(|e| format!("Failed to restore current file content: {}", e))?;

        log::info!("Staged uses: changes for file: {}", file);
        Ok(())
    }

    fn create_uses_only_version(&self, original: &str, current: &str) -> Result<String, String> {
        // Parse both versions to understand YAML structure
        let original_lines: Vec<&str> = original.lines().collect();
        let current_lines: Vec<&str> = current.lines().collect();

        // Find all uses: lines in both versions and their context
        let original_uses_info = self.extract_uses_context(&original_lines);
        let current_uses_info = self.extract_uses_context(&current_lines);

        log::debug!("Original uses: lines: {:?}", original_uses_info);
        log::debug!("Current uses: lines: {:?}", current_uses_info);

        // Start with original content as strings to avoid lifetime issues
        let mut result_lines: Vec<String> = original_lines.iter().map(|s| s.to_string()).collect();

        // Update all uses: lines that have changed by exact position matching
        for (current_idx, (current_line_num, current_uses_line)) in
            current_uses_info.iter().enumerate()
        {
            if let Some((orig_line_num, orig_uses_line)) = original_uses_info.get(current_idx) {
                // Position-based matching: same index in the list
                if *orig_line_num < result_lines.len() {
                    // Preserve original indentation by extracting it and combining with new uses content
                    let updated_line = self.preserve_indentation_with_new_uses_content(
                        orig_uses_line,
                        current_uses_line,
                    );
                    log::debug!(
                        "Updating line {} from '{}' to '{}'",
                        orig_line_num,
                        orig_uses_line,
                        updated_line
                    );
                    result_lines[*orig_line_num] = updated_line;
                }
            } else {
                log::debug!(
                    "No position match found for current line {}: {}",
                    current_line_num,
                    current_uses_line
                );
            }
        }

        Ok(result_lines.join("\n"))
    }

    fn preserve_indentation_with_new_uses_content(
        &self,
        original_line: &str,
        current_line: &str,
    ) -> String {
        // Extract indentation from original line (everything before "uses:")
        let original_indent = if let Some(uses_pos) = original_line.find("uses:") {
            &original_line[..uses_pos]
        } else {
            // Fallback: extract leading whitespace
            let trimmed = original_line.trim_start();
            &original_line[..original_line.len() - trimmed.len()]
        };

        // Extract the uses content from current line (everything from "uses:" onwards)
        let current_uses_content = if let Some(uses_pos) = current_line.find("uses:") {
            &current_line[uses_pos..]
        } else {
            current_line.trim_start() // Fallback to trimmed content
        };

        // Combine original indentation with new uses content
        format!("{}{}", original_indent, current_uses_content)
    }

    fn extract_uses_context(&self, lines: &[&str]) -> Vec<(usize, String)> {
        let mut uses_lines = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            // Check for both "uses:" and "- uses:" patterns
            if trimmed.starts_with("uses:") || trimmed.starts_with("- uses:") {
                uses_lines.push((i, line.to_string()));
            }
        }

        uses_lines
    }

    pub fn get_files_with_uses_changes(&self) -> Result<Vec<String>, String> {
        // Get diff to see what files have uses: changes
        let output = Command::new("git")
            .args(["diff", "--name-only", "HEAD"])
            .current_dir(&self.working_dir)
            .output()
            .map_err(|e| format!("Failed to get modified files: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "Git diff failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let files = String::from_utf8_lossy(&output.stdout);
        let mut uses_files = Vec::new();

        for file in files.lines() {
            if file.trim().is_empty() {
                continue;
            }

            // Check if this file has uses: changes
            let diff_output = Command::new("git")
                .args(["diff", "HEAD", "--", file])
                .current_dir(&self.working_dir)
                .output()
                .map_err(|e| format!("Failed to get diff for {}: {}", file, e))?;

            if diff_output.status.success() {
                let diff_content = String::from_utf8_lossy(&diff_output.stdout);
                // Look for lines that have uses: changes (added or removed)
                if diff_content.lines().any(|line| {
                    (line.starts_with("+") || line.starts_with("-")) && line.contains("uses:")
                }) {
                    log::info!("Found uses: changes in file: {}", file);
                    uses_files.push(file.to_string());
                }
            }
        }

        Ok(uses_files)
    }

    pub fn commit_changes(&self, message: &str) -> Result<(), String> {
        // First check if there are any staged changes
        let status_output = Command::new("git")
            .arg("diff")
            .arg("--cached")
            .arg("--name-only")
            .current_dir(&self.working_dir)
            .output()
            .map_err(|e| format!("Failed to check staged changes: {}", e))?;

        if !status_output.status.success() {
            return Err(format!(
                "Git diff --cached failed: {}",
                String::from_utf8_lossy(&status_output.stderr)
            ));
        }

        let staged_files = String::from_utf8_lossy(&status_output.stdout);
        if staged_files.trim().is_empty() {
            log::info!("No changes staged for commit");
            return Ok(());
        }

        // Commit the staged changes
        let output = Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg(message)
            .current_dir(&self.working_dir)
            .output()
            .map_err(|e| format!("Failed to execute git commit: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "Git commit failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        log::info!("Successfully committed changes");
        Ok(())
    }

    pub fn create_branch(&self, branch_name: &str) -> Result<(), String> {
        let output = Command::new("git")
            .arg("checkout")
            .arg("-b")
            .arg(branch_name)
            .current_dir(&self.working_dir)
            .output()
            .map_err(|e| format!("Failed to execute git checkout: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "Git checkout failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        log::info!(
            "Successfully created and switched to branch: {}",
            branch_name
        );
        Ok(())
    }

    pub fn checkout_branch(&self, branch_name: &str) -> Result<(), String> {
        let output = Command::new("git")
            .arg("checkout")
            .arg(branch_name)
            .current_dir(&self.working_dir)
            .output()
            .map_err(|e| format!("Failed to execute git checkout: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "Git checkout failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        log::info!("Successfully checked out branch: {}", branch_name);
        Ok(())
    }

    pub fn push_changes(&self, branch: &str, force: bool) -> Result<(), String> {
        let mut args = vec!["push", "origin", branch];
        if force {
            args.insert(1, "--force");
        }

        let output = Command::new("git")
            .args(&args)
            .current_dir(&self.working_dir)
            .output()
            .map_err(|e| format!("Failed to execute git push: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "Git push failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        log::info!("Successfully pushed changes to branch: {}", branch);
        Ok(())
    }
}

pub fn clone_repository(repo_url: &str, target_path: &str) -> Result<GitRepository, String> {
    let output = Command::new("git")
        .arg("clone")
        .arg(repo_url)
        .arg(target_path)
        .output()
        .map_err(|e| format!("Failed to execute git clone: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Git clone failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    GitRepository::open(target_path.to_string())
}
