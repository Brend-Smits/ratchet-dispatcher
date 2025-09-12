use anyhow::Result;
use clap::Parser;
use clap_verbosity_flag::Verbosity;
use github::GitHubClient;
use io::get_pr_body_from_file;
use log::{debug, error, info};
use ratchet::upgrade_workflows;
use std::{env, process};

use crate::io::cleanup_clone_dir;

mod git;
mod github;
mod io;
mod ratchet;

#[derive(Parser, Debug, Clone)]
#[clap(version)]
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
    #[clap(
        long,
        help = "Perform a dry run without pushing changes or creating pull requests"
    )]
    dry_run: bool,
    #[clap(
        long,
        help = "Clean ratchet comments to show only semantic version (e.g., '# ratchet:actions/checkout@v4' becomes '# v4')"
    )]
    clean_comment: bool,
}

fn load_env_vars() -> Result<String> {
    dotenv::dotenv().ok();
    match env::var("GITHUB_TOKEN") {
        Ok(token) => Ok(token),
        Err(_) => {
            eprintln!("GITHUB_TOKEN environment variable is not set");
            process::exit(1);
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    env_logger::Builder::new()
        .filter_level(args.verbose.log_level_filter())
        .format_module_path(false)
        .format_target(false)
        .init();

    // Check if ratchet is available early to fail fast
    info!("Checking if ratchet tool is available...");
    ratchet::check_ratchet_availability()?;

    if args.dry_run {
        info!("🔍 DRY RUN MODE: No changes will be pushed or pull requests created");
    }

    let token = load_env_vars()?;
    let repos: Vec<&str> = args.repos.split(',').collect();
    process_repositories(repos, args.clone(), token).await?;

    Ok(())
}

async fn process_repositories(repos: Vec<&str>, args: Args, token: String) -> Result<()> {
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
            GitHubClient::new(owner.to_string(), repo_name.to_string(), token.clone())?;
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

        if !args.dry_run {
            cleanup_clone_dir(&local_path);
        }
    }
    Ok(())
}

async fn process_single_repository(
    repo_url: &str,
    local_path: &str,
    args: &Args,
    github_client: &GitHubClient,
    default_branch: &str,
) -> Result<()> {
    info!("Processing repository: {}", repo_url);
    debug!("Local path: {}", local_path);
    debug!("Branch: {}", args.branch);
    debug!("Default branch: {}", default_branch);

    let git_repo = git::clone_repository(repo_url, local_path)?;

    if git_repo.checkout_branch(&args.branch).is_err() {
        debug!("Branch {} doesn't exist, creating it", args.branch);
        git_repo.create_branch(&args.branch)?;
    } else {
        debug!("Successfully checked out existing branch {}", args.branch);
    }

    debug!("Starting workflow upgrades...");
    upgrade_workflows(local_path, args.clean_comment).await?;
    debug!("Workflow upgrades completed");

    debug!("Staging changes...");
    git_repo.stage_changes()?;
    debug!("Staging completed");

    debug!("Committing changes...");
    let has_changes = if args.dry_run {
        // In dry-run mode, check if there would be changes without actually committing
        git_repo.check_staged_changes()?
    } else {
        git_repo.commit_changes("ci: pin versions of workflow actions")?
    };

    if !has_changes {
        info!(
            "No changes to commit for repository {}, skipping PR creation",
            repo_url
        );
        return Ok(());
    }

    debug!("Changes committed successfully");

    if args.dry_run {
        info!(
            "🔍 DRY RUN: Would push changes to branch '{}' and create/update PR for repository {}",
            args.branch, repo_url
        );
        info!("🔍 DRY RUN: Changes that would be committed:");

        // Show the diff that would be committed
        if let Err(e) = git_repo.show_staged_diff() {
            debug!("Could not show staged diff: {}", e);
        }

        info!("🔍 DRY RUN: Repository clone preserved at: {}", local_path);
        return Ok(());
    }

    let force_push = match github_client.find_existing_pr(&args.branch).await {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(e) => {
            error!("Failed to check existing PR: {}", e);
            return Err(e);
        }
    };

    git_repo.push_changes(&args.branch, true)?;

    if !force_push {
        match github_client
            .create_pull_request(
                &args.branch,
                default_branch.to_owned(),
                get_pr_body_from_file(&args.pr_body_path)?,
            )
            .await
        {
            Ok(pr) => {
                if let Some(html_url) = pr.html_url {
                    info!("Created PR for {}: {}", repo_url, html_url);
                } else {
                    info!("Created PR for {}: (URL not available)", repo_url);
                }
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
