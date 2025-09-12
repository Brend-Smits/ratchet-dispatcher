use anyhow::{Context, Result};
use octocrab::{models::pulls::PullRequest, Octocrab};

pub struct GitHubClient {
    octocrab: Octocrab,
    owner: String,
    repo: String,
}

impl GitHubClient {
    pub fn new(owner: String, repo: String, token: String) -> Result<Self> {
        let octocrab = Octocrab::builder().personal_token(token).build()?;
        Ok(GitHubClient {
            octocrab,
            owner,
            repo,
        })
    }

    // Make a request to the GitHub API to create a pull request
    // with the given branch, default branch, and pull request body
    // Return the created pull request
    pub async fn create_pull_request(
        &self,
        branch: &str,
        default_branch: String,
        pr_body: String,
    ) -> Result<PullRequest> {
        let pr = self
            .octocrab
            .pulls(&self.owner, &self.repo)
            .create("ci: pin versions of actions", branch, default_branch)
            .body(pr_body)
            .maintainer_can_modify(true)
            .send()
            .await
            .with_context(|| {
                format!(
                    "Failed to create pull request for branch '{}' in repository '{}/{}'",
                    branch, self.owner, self.repo
                )
            })?;
        Ok(pr)
    }

    // Make a request to the GitHub API to find an existing pull request
    // with the given branch
    // Return the pull request if it exists, otherwise return None
    pub async fn find_existing_pr(&self, branch: &str) -> Result<Option<PullRequest>> {
        let pulls = self
            .octocrab
            .pulls(&self.owner, &self.repo)
            .list()
            .head(format!("{}:{}", &self.owner, branch))
            .state(octocrab::params::State::Open)
            .send()
            .await
            .with_context(|| {
                format!(
                    "Failed to list pull requests for branch '{}' in repository '{}/{}'",
                    branch, self.owner, self.repo
                )
            })?;

        Ok(pulls.items.into_iter().next())
    }

    // Make a request to the GitHub API to get the default branch of the repository
    // Return the default branch
    pub async fn get_default_branch(&self) -> Result<String> {
        let repo = self
            .octocrab
            .repos(&self.owner, &self.repo)
            .get()
            .await
            .with_context(|| {
                format!(
                    "Failed to get repository information for '{}/{}'",
                    self.owner, self.repo
                )
            })?;
        Ok(repo.default_branch.unwrap_or_else(|| "main".to_string()))
    }
}
