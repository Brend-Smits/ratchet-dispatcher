use std::env;

use git2::{ApplyOptions, Cred, DiffOptions, PushOptions, RemoteCallbacks, Repository};
use log::info;

pub struct GitRepository {
    repo: Repository,
}

impl GitRepository {
    // Function that will do the following command:
    // git clone <repo_url> <local_path>
    // This will clone the repository from <repo_url> to <local_path>
    // Local path is usually a temporary directory.
    pub fn clone_repo(
        repo_url: &str,
        local_path: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Cloning repository from {} to {}", repo_url, local_path);

        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, _username_from_url, _allowed_types| {
            let token = env::var("GITHUB_TOKEN").unwrap_or_else(|_| String::from("default_token"));
            Cred::userpass_plaintext("x-access-token", &token)
        });

        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        // Prepare builder
        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_options);

        let repo = builder.clone(repo_url, std::path::Path::new(local_path))?;

        Ok(GitRepository { repo })
    }

    // Function that will do the following command:
    // git branch <branch> <commit>
    // This will create a new branch with the name <branch>
    pub fn create_branch(&self, branch: &str) -> Result<(), Box<dyn std::error::Error>> {
        let head = self.repo.head()?;
        let commit = head.peel_to_commit()?;
        self.repo.branch(branch, &commit, false)?;
        Ok(())
    }

    // Function that will do the following command:
    // git diff -U0 -w --no-color --ignore-blank-lines | git apply --cached --ignore-whitespace --unidiff-zero -
    // This will essentially remove only the blank line changes from the changes
    // This is a hack as we don't like it that Ratchet 'cleans' up the workflow files.
    // Ratchet by default removes the blank lines after a workflow step.
    // This is not something we want to do as it makes the workflow files harder to read.
    pub fn remove_blank_line_changes(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut diff_options = DiffOptions::new();
        diff_options
            .ignore_whitespace(true)
            .ignore_blank_lines(true)
            .context_lines(0);

        let diff = self
            .repo
            .diff_index_to_workdir(None, Some(&mut diff_options))?;

        let mut apply_options = ApplyOptions::new();
        apply_options.hunk_callback(|_hunk| true);
        self.repo
            .apply(&diff, git2::ApplyLocation::Index, Some(&mut apply_options))?;

        Ok(())
    }

    // Function that will stage all the changes in the .github/workflows directory ignoring whitespace and blank line changes
    pub fn stage_changes(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut diff_options = DiffOptions::new();
        diff_options
            .ignore_whitespace(true)
            .ignore_blank_lines(true)
            .pathspec(".github/workflows")
            .pathspec(".github/workflows/*");

        let diff = self
            .repo
            .diff_index_to_workdir(None, Some(&mut diff_options))?;

        let mut apply_options = ApplyOptions::new();
        apply_options.hunk_callback(|_hunk| true);
        self.repo
            .apply(&diff, git2::ApplyLocation::Index, Some(&mut apply_options))?;

        Ok(())
    }

    // Function that will do the following command:
    // git add .github/workflows/*
    // git commit -m "ci: pin versions of workflow actions"
    // This will add all the changes in the .github/workflows directory and commit them with the message "ci: pin versions of workflow actions"
    pub fn commit_changes(&self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut index = self.repo.index()?;
        index.add_all(
            [".github/workflows/*"].iter(),
            git2::IndexAddOption::DEFAULT,
            None,
        )?;
        index.write()?;
        let tree_id = index.write_tree()?;
        let tree = self.repo.find_tree(tree_id)?;
        let parent_commit = self.repo.head()?.peel_to_commit()?;
        let signature = self.repo.signature()?;
        self.repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[&parent_commit],
        )?;
        Ok(())
    }

    // Function that will do the following command:
    // git push origin <branch>
    // This will push the changes to the remote repository
    pub fn push_changes(
        &self,
        branch: &str,
        force: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut remote = self.repo.find_remote("origin")?;
        let refspec = if force {
            format!("+refs/heads/{}:refs/heads/{}", branch, branch)
        } else {
            format!("refs/heads/{}:refs/heads/{}", branch, branch)
        };

        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, _username_from_url, _allowed_types| {
            //TODO: This should not be here
            let token = env::var("GITHUB_TOKEN").unwrap_or_else(|_| String::from("default_token"));
            Cred::userpass_plaintext("x-access-token", &token)
        });

        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(callbacks);

        remote.push(&[&refspec], Some(&mut push_options))?;
        Ok(())
    }

    // Function that will do the following command:
    // git rev-parse --verify refs/heads/<branch>
    // If the branch does not exist it will create the branch
    // If the branch exists it will checkout the branch
    pub fn checkout_branch(&self, branch: &str) -> Result<(), Box<dyn std::error::Error>> {
        let obj = match self.repo.revparse_single(&format!("refs/heads/{}", branch)) {
            Ok(obj) => obj,
            Err(_) => {
                self.create_branch(branch)?;
                self.repo
                    .revparse_single(&format!("refs/heads/{}", branch))?
            }
        };
        self.repo.checkout_tree(&obj, None)?;
        self.repo.set_head(&format!("refs/heads/{}", branch))?;
        Ok(())
    }
}
