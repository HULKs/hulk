use clap::{Args, Command};
use clap_complete::{generate, Shell};
use color_eyre::Result;
use regex::Regex;

use crate::aliveness::completions as complete_naos;

#[derive(Args)]
pub struct Arguments {
    #[arg(long, hide = true)]
    complete_naos: bool,
    #[clap(name = "shell")]
    pub shell: clap_complete::shells::Shell,
}

pub async fn completions(arguments: Arguments, mut command: Command) -> Result<()> {
    if arguments.complete_naos {
        let naos = complete_naos().await?;

        let separator = match arguments.shell {
            Shell::Bash => ' ',
            Shell::Fish => '\n',
            _ => ' ',
        };

        for nao in naos {
            print!("{nao}{separator}");
        }
        return Ok(());
    }

    let mut static_completion = Vec::new();
    generate(
        arguments.shell,
        &mut command,
        "pepsi",
        &mut static_completion,
    );

    let static_completions = String::from_utf8(static_completion)?;
    dynamic_completions(arguments.shell, static_completions);
    Ok(())
}

fn dynamic_completions(shell: Shell, static_completions: String) {
    let completion_cmd = format!("pepsi completions --complete-naos {shell}");

    match shell {
        Shell::Bash => {
            let re = Regex::new("(?:<NAOS>|\\[NAOS\\])...").unwrap();
            let completions = re.replace_all(&static_completions, format!("$({completion_cmd})"));
            print!("{completions}")
        }
        Shell::Fish => {
            const COMPLETION_SUBCOMMANDS: [(&str, &str); 11] = [
                ("aliveness", ""),
                ("hulk", ""),
                ("logs", "delete"),
                ("logs", "downloads"),
                ("postgame", ""),
                ("poweroff", ""),
                ("reboot", ""),
                ("upload", ""),
                ("wireless", "list"),
                ("wireless", "set"),
                ("wireless", "status"),
            ];
            print!("{static_completions}");
            for (first, second) in COMPLETION_SUBCOMMANDS {
                if second.is_empty() {
                    println!(
                        "complete -c pepsi -n \"__fish_seen_subcommand_from {first}\" \
                             -f -a \"({completion_cmd})\""
                    );
                } else {
                    println!(
                        "complete -c pepsi -n \"__fish_seen_subcommand_from {first}; \
                             and __fish_seen_subcommand_from {second}\" \
                             -f -a \"({completion_cmd})\""
                    );
                }
            }
        }
        _ => print!("{static_completions}"),
    };
}
