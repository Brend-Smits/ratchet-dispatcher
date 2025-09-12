use clap::Parser;
use clap_verbosity_flag::Verbosity;
use github::GitHubClient;
use io::get_pr_body_from_file;
use log::{error, info};
use ratchet::upgrade_workflows;
use std::{env, error::Error, process};

use crate::io::cleanup_clone_dir;

mod git;
mod github;
mod io;
mod ratchet;

#[derive(Parser, Debug, Clone)]
struct Args {
    #[clap(long)]
    repos: String,
    #[clap(long, default_value = "automated-ratchet-dispatcher-pin")]
    branch: String,
    #[clap(flatten)]
    verbose: Verbosity,
    #[clap(long, default_value = "temp_clones")]
    clone_dir: String,
    #[clap(long)]
    pr_body_path: Option<String>,
}

fn load_env_vars() -> String {
    dotenv::dotenv().ok();
    match env::var("GITHUB_TOKEN") {
        Ok(token) => token,
        Err(_) => {
            eprintln!("GITHUB_TOKEN environment variable is not set");
            process::exit(1);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    env_logger::Builder::new()
        .filter_level(args.verbose.log_level_filter())
        .format_module_path(false)
        .format_target(false)
        .init();
    let token = load_env_vars();
    let repos: Vec<&str> = args.repos.split(',').collect();
    process_repositories(repos, args.clone(), token).await;

    Ok(())
}

async fn process_repositories(repos: Vec<&str>, args: Args, token: String) {
    for repo in repos {
        let repo_parts: Vec<&str> = repo.split('/').collect();
        if repo_parts.len() != 2 {
            error!("Invalid repository format: {}", repo);
            continue;
        }
        let owner = repo_parts[0];
        let repo_name = repo_parts[1];
        let repo_url = format!("https://github.com/{}/{}.git", owner, repo_name);
        let local_path = format!("{}/{}_{}", args.clone_dir, owner, repo_name);
        let github_client =
            GitHubClient::new(owner.to_string(), repo_name.to_string(), token.clone());
        let default_branch = match github_client.get_default_branch().await {
            Ok(branch) => branch,
            Err(e) => {
                error!("Failed to get default branch: {}", e);
                continue;
            }
        };
        if let Err(e) = process_single_repository(
            &repo_url,
            &local_path,
            &args,
            &github_client,
            &default_branch,
        )
        .await
        {
            error!("Failed to process repository {}: {}", repo, e);
        }
        cleanup_clone_dir(&local_path);
    }
}

async fn process_single_repository(
    repo_url: &str,
    local_path: &str,
    args: &Args,
    github_client: &GitHubClient,
    default_branch: &str,
) -> Result<(), Box<dyn Error>> {
    let git_repo = match git::clone_repository(repo_url, local_path) {
        Ok(repo) => repo,
        Err(e) => {
            error!("Failed to clone repository: {}", e);
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)));
        }
    };

    if git_repo.checkout_branch(&args.branch).is_err() {
        if let Err(e) = git_repo.create_branch(&args.branch) {
            error!("Failed to create branch: {}", e);
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)));
        }
    }

    if let Err(e) = upgrade_workflows(local_path).await {
        error!("Failed to upgrade workflows: {}", e);
        return Err(e);
    }

    if let Err(e) = git_repo.stage_changes() {
        error!("Failed to stage changes: {}", e);
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)));
    }

    if let Err(e) = git_repo.commit_changes("ci: pin versions of workflow actions") {
        error!("Failed to commit changes: {}", e);
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)));
    }

    let force_push = match github_client.find_existing_pr(&args.branch).await {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(e) => {
            error!("Failed to check existing PR: {}", e);
            return Err(e);
        }
    };

    if let Err(e) = git_repo.push_changes(&args.branch, true) {
        error!("Failed to push changes to branch {}: {}", &args.branch, e);
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)));
    }

    if !force_push {
        match github_client
            .create_pull_request(
                &args.branch,
                default_branch.to_owned(),
                get_pr_body_from_file(&args.pr_body_path),
            )
            .await
        {
            Ok(pr) => {
                info!(
                    "Created PR for {}: {:?}",
                    repo_url,
                    format!(
                        "{}://{}/{}",
                        pr.html_url.clone().unwrap().scheme().to_string(),
                        pr.html_url.clone().unwrap().domain().unwrap().to_string(),
                        pr.html_url.unwrap().path().to_string()
                    )
                );
                Ok(())
            }
            Err(e) => {
                error!("Failed to create PR: {}", e);
                Err(e)
            }
        }
    } else {
        info!("Updated existing PR for {}", repo_url);
        Ok(())
    }
}
