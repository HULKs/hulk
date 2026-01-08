use clap::{Args, Command};
use clap_complete::{generate, Shell};
use color_eyre::Result;
use regex::Regex;

use crate::{aliveness::completions as complete_robots, cargo::MANIFEST_PATHS};

#[derive(Args)]
pub struct Arguments {
    #[arg(long, hide = true)]
    complete_robots: bool,
    #[arg(long, hide = true)]
    complete_assignments: bool,
    #[clap(name = "shell")]
    pub shell: clap_complete::shells::Shell,
}

pub async fn completions(arguments: Arguments, mut command: Command) -> Result<()> {
    if arguments.complete_robots || arguments.complete_assignments {
        let robots = complete_robots().await?;

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

        for robot in robots {
            print!("{robot}{colon}{separator}");
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
    let robot_completion_command = format!("pepsi completions --complete-robots {shell}");
    let assignment_completion_command = format!("pepsi completions --complete-assignments {shell}");

    match shell {
        Shell::Bash => {
            let re = Regex::new("(?:<RobotS?>|\\[RobotS\\])(.{3})?").unwrap();
            let completions = re.replace_all(
                &static_completions,
                format!("$({robot_completion_command})"),
            );

            let re = Regex::new("<ASSIGNMENTS>...").unwrap();
            let completions =
                re.replace_all(&completions, format!("$({assignment_completion_command})"));

            print!("{completions}")
        }
        Shell::Fish => {
            print!("{static_completions}");

            const ROBOT_COMPLETION_SUBCOMMANDS: [(&str, &str); 18] = [
                ("aliveness", ""),
                ("gammaray", ""),
                ("hulk", ""),
                ("logs", "delete"),
                ("logs", "downloads"),
                ("logs", "show"),
                ("ping", ""),
                ("postgame", "golden-goal"),
                ("postgame", "first-half"),
                ("postgame", "second-half"),
                ("poweroff", ""),
                ("pregame", ""),
                ("reboot", ""),
                ("shell", ""),
                ("upload", ""),
                ("wifi", "list"),
                ("wifi", "set"),
                ("wifi", "status"),
            ];
            for (subcommand, argument) in ROBOT_COMPLETION_SUBCOMMANDS {
                if argument.is_empty() {
                    println!(
                        "complete -c pepsi -n \"__fish_pepsi_using_subcommand {subcommand}\" \
                             -f -a \"({robot_completion_command})\""
                    );
                } else {
                    println!(
                        "complete -c pepsi -n \"__fish_pepsi_using_subcommand {subcommand}; \
                             and __fish_seen_subcommand_from {argument}\" \
                             -f -a \"({robot_completion_command})\""
                    );
                }
            }

            println!(
                "complete -c pepsi -n \"__fish_seen_subcommand_from playernumber\" \
                     -f -a \"({assignment_completion_command})\""
            );
            println!(
                "complete -c pepsi -n \"__fish_seen_subcommand_from postgame; \
                     and not __fish_seen_subcommand_from golden-goal first-half second-half\" \
                     -f -a \"golden-goal first-half second-half\""
            );

            const MANIFEST_COMPLETION_SUBCOMMANDS: [&str; 6] =
                ["build", "check", "clippy", "install", "run", "test"];
            let manifest_paths: Vec<_> = MANIFEST_PATHS.keys().copied().collect();
            for subcommand in MANIFEST_COMPLETION_SUBCOMMANDS {
                println!(
                    "complete -c pepsi -n \"__fish_pepsi_using_subcommand {subcommand}; \
                        and not __fish_seen_subcommand_from {manifest_paths}\" \
                         -f -a \"{manifest_paths}\"",
                    manifest_paths = manifest_paths.join(" ")
                );
            }
        }
        Shell::Zsh => {
            let re = Regex::new("(:robots? -- .*):").unwrap();
            let completions = re.replace_all(&static_completions, "$1:_pepsi__complete_robots");

            let re = Regex::new("(:assignments -- .*):").unwrap();
            let completions = re.replace_all(&completions, "$1:_pepsi__complete_assignments");

            println!(
                "{completions}\
                \n\
                (( $+functions[_pepsi__complete_robots] )) ||\n\
                _pepsi__complete_robots() {{\n    \
                    local commands; commands=(\"${{(@f)$({robot_completion_command})}}\")\n    \
                    _describe -t commands 'pepsi complete robots' commands \"$@\"\n\
                }}\n\
                (( $+functions[_pepsi__complete_assignments] )) ||\n\
                _pepsi__complete_assignments() {{\n    \
                    local commands; commands=(\"${{(@f)$({assignment_completion_command})}}\")\n    \
                    _describe -t commands 'pepsi complete assignments' commands \"$@\"\n\
                }}"
            );
        }
        _ => print!("{static_completions}"),
    };
}
