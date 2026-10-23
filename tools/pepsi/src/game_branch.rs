use std::path::Path;

use clap::Args;
use color_eyre::{
    Result,
    eyre::{WrapErr, bail},
};
use repository::{Repository, location::LocationTarget};
use tokio::io::{AsyncBufReadExt, BufReader, Lines, Stdin, stdin};

use crate::{
    deploy_config::DeployConfig,
    git::{
        add_all, check_clean, cherry_pick, conflicted_files, create_and_switch_to_branch,
        create_commit, merge_squash, reset, resolve_revision, select_commits,
        stage_resolved_conflicts,
    },
    player_number::{Arguments as PlayerNumberArguments, player_number},
};

#[derive(Args)]
pub struct Arguments {
    /// Create the game branch even if it already exists
    #[arg(short, long)]
    pub force: bool,
}

enum Operation {
    Merge(Vec<String>),
    CherryPick(Vec<String>),
}

async fn resolve_operation(root: &Path, text: &str) -> Result<Operation> {
    let words: Vec<_> = text.split_whitespace().collect();
    let (pick, mut revisions) = match words.as_slice() {
        ["merge", revisions @ ..] => (false, revisions),
        ["cherry-pick", revisions @ ..] => (true, revisions),
        revisions => (false, revisions),
    };
    let top = if pick
        && let Some(count) = revisions
            .first()
            .and_then(|word| word.strip_prefix("--top="))
    {
        let count: usize = count.parse().wrap_err("--top must be a positive integer")?;
        if count == 0 {
            bail!("--top must be a positive integer");
        }
        revisions = &revisions[1..];
        if revisions.len() != 1 {
            bail!("--top requires exactly one ref");
        }
        Some(count)
    } else {
        None
    };
    if revisions.is_empty() || revisions.iter().any(|revision| revision.starts_with('-')) {
        bail!("expected refs, without Git options");
    }
    if pick {
        let commits = if top.is_some() {
            let revision = resolve_revision(root, revisions[0]).await?;
            select_commits(root, &[&revision], top).await?
        } else {
            select_commits(root, revisions, None).await?
        };
        Ok(Operation::CherryPick(commits))
    } else {
        let mut commits = Vec::new();
        for revision in revisions {
            commits.push(resolve_revision(root, revision).await?);
        }
        Ok(Operation::Merge(commits))
    }
}

pub async fn game_branch(arguments: Arguments, repository: &Repository) -> Result<()> {
    let config = DeployConfig::read_from_file(repository)
        .await
        .wrap_err("failed to read deploy config from file")?;

    let root = &repository.root;
    check_clean(root).await?;
    let base = resolve_revision(root, &config.base)
        .await
        .wrap_err_with(|| format!("can't find base branch: {}", config.base))?;
    let mut groups = Vec::new();
    for (index, entry) in config.branches.iter().enumerate() {
        if entry.operations().is_empty() {
            bail!("branches[{}] is an empty group", index);
        }
        let mut group = Vec::new();
        for text in entry.operations() {
            let operation = resolve_operation(root, text)
                .await
                .wrap_err_with(|| format!("invalid branches[{index}] operation: {text}"))?;
            group.push((text.as_str(), operation));
        }
        groups.push(group);
    }

    let branch_name = config.branch_name();
    create_and_switch_to_branch(root, &branch_name, &base, arguments.force)
        .await
        .wrap_err("failed to create and switch to branch")?;

    let mut input = BufReader::new(stdin()).lines();
    for (index, group) in groups.iter().enumerate() {
        let start = resolve_revision(root, "HEAD").await?;
        let result = apply_group(root, group, &start, &mut input)
            .await
            .wrap_err_with(|| format!("failed to apply branches[{index}]"));
        if !matches!(&result, Ok(true)) {
            reset(root, &start, false)
                .await
                .wrap_err("failed to roll back group")?;
        }
        result?;
    }

    configure_repository(repository, config).await?;
    add_all(root).await?;
    create_commit(root, "Add player number assigments and deployment config")
        .await
        .wrap_err("failed to create commit")?;

    Ok(())
}

async fn apply_group(
    root: &Path,
    group: &[(&str, Operation)],
    start: &str,
    input: &mut Lines<BufReader<Stdin>>,
) -> Result<bool> {
    let description = group
        .iter()
        .map(|(text, _)| *text)
        .collect::<Vec<_>>()
        .join("; ");
    for (text, operation) in group {
        let commits = match operation {
            Operation::Merge(commits) | Operation::CherryPick(commits) => commits,
        };
        for commit in commits {
            let result = match operation {
                Operation::Merge(_) => merge_squash(root, commit).await,
                Operation::CherryPick(_) => cherry_pick(root, commit).await,
            };
            if let Err(error) = result {
                let conflicts = conflicted_files(root).await?;
                if conflicts.is_empty() {
                    return Err(error)
                        .wrap_err_with(|| format!("operation failed: {text} ({commit})"));
                }
                eprintln!("Conflicts in '{text}' at {commit}.");
                loop {
                    if confirmation_prompt(
                        &format!("Do you want to skip deploying the entire group '{description}'?"),
                        input,
                    )
                    .await?
                    {
                        return Ok(false);
                    }
                    eprintln!(
                        "Please resolve all conflicts now. Resolved files will be staged automatically."
                    );
                    if confirmation_prompt("Were you able to resolve the conflicts?", input).await?
                    {
                        if stage_resolved_conflicts(root, &conflicts).await? {
                            break;
                        }
                        eprintln!("There are still unresolved conflicts.");
                    }
                }
            }
            // Keep the index clean so subsequent merges can run after cherry-picks.
            create_commit(root, &format!("gamebranch intermediate: {text} ({commit})")).await?;
        }
    }
    reset(root, start, true).await?;
    create_commit(root, &format!("merge-squash {description}"))
        .await
        .wrap_err("failed to create group commit")?;
    Ok(true)
}

async fn configure_repository(repository: &Repository, config: DeployConfig) -> Result<()> {
    repository
        .set_location(LocationTarget::Default, &config.location)
        .await
        .wrap_err_with(|| format!("failed to set location to {}", config.location))?;

    player_number(
        PlayerNumberArguments {
            assignments: config
                .assignments
                .iter()
                .copied()
                .map(TryFrom::try_from)
                .collect::<Result<Vec<_>, _>>()?,
        },
        repository,
    )
    .await
    .wrap_err("failed to set player numbers")?;

    Ok(())
}

async fn confirmation_prompt(message: &str, lines: &mut Lines<BufReader<Stdin>>) -> Result<bool> {
    loop {
        eprint!("{message} [y/n] ");

        if let Some(line) = lines
            .next_line()
            .await
            .wrap_err("failed to get next line")?
        {
            match line {
                line if line == "y" => return Ok(true),
                line if line == "n" => return Ok(false),
                _ => {}
            }
        } else {
            bail!("input closed while waiting for conflict resolution");
        }
    }
}
