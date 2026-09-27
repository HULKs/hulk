use clap::Command;

const CATEGORIES: [&str; 5] = [
    "Development",
    "Simulation",
    "Robot Operations",
    "Game Workflow",
    "Utilities",
];

fn category(name: &str) -> &'static str {
    match name {
        "build" | "check" | "clippy" | "format" | "hydra-bench" | "install" | "nextest" | "run"
        | "tensor-rt-compile" | "test" => "Development",
        "mujoco-viewer" => "Simulation",
        "aliveness" | "boosterize" | "gammaray" | "hulk" | "log" | "ping" | "poweroff"
        | "reboot" | "sdk" | "shell" | "upload" | "wifi" => "Robot Operations",
        "gamebranch" | "location" | "playernumber" | "postgame" | "pregame" => "Game Workflow",
        _ => "Utilities",
    }
}

pub fn grouped(mut command: Command) -> Command {
    command.build();
    let heading_style = command.get_styles().get_header();
    let mut template = String::from("{before-help}{about-with-newline}{usage-heading} {usage}\n");

    // Clap has one subcommand heading. Render each category through Clap so
    // descriptions, aliases, wrapping, and styling still come from its metadata.
    for heading in CATEGORIES {
        let mut section = command
            .clone()
            .help_template("{subcommands}")
            .mut_subcommands(|subcommand| {
                let hidden = subcommand.is_hide_set() || category(subcommand.get_name()) != heading;
                subcommand.hide(hidden).display_order(0)
            });
        template.push_str(&format!(
            "\n{heading_style}{heading}:{heading_style:#}\n{}",
            section.render_help().ansi(),
        ));
    }
    template.push_str(&format!(
        "\n{heading_style}Options:{heading_style:#}\n{{options}}{{after-help}}",
    ));
    command.help_template(template)
}
