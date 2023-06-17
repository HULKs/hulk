use clap::{Args, Command};
use clap_complete::{generate, Shell};
use color_eyre::Result;
use regex::Regex;

use crate::aliveness::completions as complete_naos;

#[derive(Args)]
pub struct Arguments {
    #[arg(long, hide = true)]
    complete_naos: bool,
    #[arg(long, hide = true)]
    complete_assignments: bool,
    #[clap(name = "shell")]
    pub shell: clap_complete::shells::Shell,
}

pub async fn completions(arguments: Arguments, mut command: Command) -> Result<()> {
    if arguments.complete_naos || arguments.complete_assignments {
        let naos = complete_naos().await?;

        let separator = match arguments.shell {
            Shell::Bash => ' ',
            Shell::Fish => '\n',
            Shell::Zsh => '\n',
            _ => ' ',
        };
        let colon = match arguments.complete_assignments {
            true => match arguments.shell {
                Shell::Zsh => "\\:",
                _ => ":",
            },
            false => "",
        };

        for nao in naos {
            print!("{nao}{colon}{separator}");
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
    let nao_completion_command = format!("pepsi completions --complete-naos {shell}");
    let assignement_completion_command =
        format!("pepsi completions --complete-assignments {shell}");

    match shell {
        Shell::Bash => {
            let re = Regex::new("(?:<NAOS?>|\\[NAOS\\])(.{3})?").unwrap();
            let completions =
                re.replace_all(&static_completions, format!("$({nao_completion_command})"));

            let re = Regex::new("<ASSIGNMENTS>...").unwrap();
            let completions =
                re.replace_all(&completions, format!("$({assignement_completion_command})"));

            print!("{completions}")
        }
        Shell::Fish => {
            print!("{static_completions}");

            const COMPLETION_SUBCOMMANDS: [(&str, &str); 14] = [
                ("aliveness", ""),
                ("gammaray", ""),
                ("hulk", ""),
                ("logs", "delete"),
                ("logs", "downloads"),
                ("logs", "show"),
                ("postgame", ""),
                ("poweroff", ""),
                ("reboot", ""),
                ("shell", ""),
                ("upload", ""),
                ("wireless", "list"),
                ("wireless", "set"),
                ("wireless", "status"),
            ];
            for (first, second) in COMPLETION_SUBCOMMANDS {
                if second.is_empty() {
                    println!(
                        "complete -c pepsi -n \"__fish_seen_subcommand_from {first}\" \
                             -f -a \"({nao_completion_command})\""
                    );
                } else {
                    println!(
                        "complete -c pepsi -n \"__fish_seen_subcommand_from {first}; \
                             and __fish_seen_subcommand_from {second}\" \
                             -f -a \"({nao_completion_command})\""
                    );
                }
            }

            const ASSIGNEMNT_COMPLETION_SUBCOMMANDS: [&str; 2] = ["playernumber", "pregame"];
            for subcommand in ASSIGNEMNT_COMPLETION_SUBCOMMANDS {
                println!(
                    "complete -c pepsi -n \"__fish_seen_subcommand_from {subcommand}\" \
                         -f -a \"({assignement_completion_command})\""
                );
            }
        }
        Shell::Zsh => {
            let re = Regex::new("(:naos? -- .*):").unwrap();
            let completions = re.replace_all(&static_completions, "$1:_pepsi__complete_naos");

            let re = Regex::new("(:assignments -- .*):").unwrap();
            let completions = re.replace_all(&completions, "$1:_pepsi__complete_assignments");

            println!(
                "{completions}\
                \n\
                (( $+functions[_pepsi__complete_naos] )) ||\n\
                _pepsi__complete_naos() {{\n    \
                    local commands; commands=(\"${{(@f)$({nao_completion_command})}}\")\n    \
                    _describe -t commands 'pepsi complete naos' commands \"$@\"\n\
                }}\n\
                (( $+functions[_pepsi__complete_assignments] )) ||\n\
                _pepsi__complete_assignments() {{\n    \
                    local commands; commands=(\"${{(@f)$({assignement_completion_command})}}\")\n    \
                    _describe -t commands 'pepsi complete assignments' commands \"$@\"\n\
                }}"
            );
        }
        _ => print!("{static_completions}"),
    };
}
