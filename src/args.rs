use std::path::PathBuf;

#[derive(clap::Parser, Clone, Debug)]
pub enum CustomCommand {
    #[command()]
    Starship(StarshipOptions),
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum StarshipCommands {
    /// Print the configured Prompt
    Prompt {
        /// Path to the jj-starship config file
        #[arg(long, env = "STARSHIP_JJ_CONFIG")]
        starship_config: Option<PathBuf>,
    },

    /// Interact with the configuration
    #[command(subcommand)]
    Config(ConfigCommands),
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum ConfigCommands {
    /// Print the path to the config file
    Path,
    /// Print the default Config
    Default,
}

#[derive(clap::Args, Clone, Debug)]
pub struct StarshipOptions {
    #[command(subcommand)]
    pub command: StarshipCommands,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser as _;

    #[test]
    fn parses_prompt_with_config_path() {
        let cmd = CustomCommand::parse_from([
            "prog",
            "starship",
            "prompt",
            "--starship-config",
            "foo.toml",
        ]);
        match cmd {
            CustomCommand::Starship(StarshipOptions { command }) => match command {
                StarshipCommands::Prompt { starship_config } => {
                    assert_eq!(
                        starship_config.as_deref(),
                        Some(std::path::Path::new("foo.toml"))
                    )
                }
                _ => panic!("expected prompt"),
            },
        }
    }

    #[test]
    fn parses_config_path_subcommand() {
        let cmd = CustomCommand::parse_from(["prog", "starship", "config", "path"]);
        match cmd {
            CustomCommand::Starship(StarshipOptions { command }) => match command {
                StarshipCommands::Config(ConfigCommands::Path) => {}
                _ => panic!("expected config path"),
            },
        }
    }

    #[test]
    fn parses_config_default_subcommand() {
        let cmd = CustomCommand::parse_from(["prog", "starship", "config", "default"]);
        match cmd {
            CustomCommand::Starship(StarshipOptions { command }) => match command {
                StarshipCommands::Config(ConfigCommands::Default) => {}
                _ => panic!("expected config default"),
            },
        }
    }
}
