use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::{
    dev::package_dev_stdlib,
    fsutil::exit_code,
    home::{home_dir, RockupHome},
    shell::{
        ensure_shell_setup, ensure_shims, print_current_shell_activation_hint, render_env_script,
    },
    target::add_target_component,
    toolchain::{
        install_toolchain, list_toolchains, proxy_toolchain_command, remove_toolchain,
        run_toolchain_command, set_default_toolchain,
    },
};

pub(crate) fn run() -> Result<Exit, String> {
    let config = Config::parse();
    let home = RockupHome::resolve()?;

    match config.command {
        CommandConfig::Toolchain { command } => match command {
            ToolchainCommand::Install { name, path } => {
                let installed_path = install_toolchain(&home, &name, &path)?;
                ensure_shell_setup(&home)?;
                print_current_shell_activation_hint()?;
                println!("{}", installed_path.display());
                Ok(Exit::Success)
            }
            ToolchainCommand::Remove { name } => {
                let removed_path = remove_toolchain(&home, &name)?;
                println!("{}", removed_path.display());
                Ok(Exit::Success)
            }
            ToolchainCommand::List => {
                for toolchain in list_toolchains(&home)? {
                    let prefix = if toolchain.is_active { "*" } else { " " };
                    println!("{} {}", prefix, toolchain.name);
                }
                Ok(Exit::Success)
            }
        },
        CommandConfig::Target { command } => match command {
            TargetCommand::Add {
                triple,
                path,
                toolchain,
            } => {
                let installed_path =
                    add_target_component(&home, &triple, toolchain.as_deref(), &path)?;
                println!("{}", installed_path.display());
                Ok(Exit::Success)
            }
        },
        CommandConfig::Dev { command } => match command {
            DevCommand::Stdlib { command } => match command {
                DevStdlibCommand::Package {
                    path,
                    sysroot,
                    target,
                    copy_source,
                    rockc,
                } => {
                    let packaged =
                        package_dev_stdlib(&path, &sysroot, target, copy_source, rockc.as_deref())?;
                    println!("{}", packaged.display());
                    Ok(Exit::Success)
                }
            },
        },
        CommandConfig::Default { name } => {
            set_default_toolchain(&home, &name)?;
            ensure_shims(&home)?;
            ensure_shell_setup(&home)?;
            print_current_shell_activation_hint()?;
            Ok(Exit::Success)
        }
        CommandConfig::Env => {
            print!("{}", render_env_script(&home, &home_dir()?));
            Ok(Exit::Success)
        }
        CommandConfig::Run { name, command } => {
            let status = run_toolchain_command(&home, &name, &command)?;
            Ok(Exit::Code(exit_code(status)))
        }
        CommandConfig::Proxy { binary, args } => {
            let status = proxy_toolchain_command(&home, &binary, &args)?;
            Ok(Exit::Code(exit_code(status)))
        }
    }
}

pub(crate) enum Exit {
    Success,
    Code(i32),
}

#[derive(Parser, Debug)]
#[command(version, about = "Rock toolchain manager", long_about = None)]
pub(crate) struct Config {
    #[command(subcommand)]
    pub(crate) command: CommandConfig,
}

#[derive(Subcommand, Debug)]
pub(crate) enum CommandConfig {
    Toolchain {
        #[command(subcommand)]
        command: ToolchainCommand,
    },
    Target {
        #[command(subcommand)]
        command: TargetCommand,
    },
    Dev {
        #[command(subcommand)]
        command: DevCommand,
    },
    Default {
        name: String,
    },
    Env,
    Run {
        name: String,
        #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
    #[command(hide = true)]
    Proxy {
        binary: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum ToolchainCommand {
    Install {
        name: String,
        #[arg(long)]
        path: PathBuf,
    },
    Remove {
        name: String,
    },
    List,
}

#[derive(Subcommand, Debug)]
pub(crate) enum TargetCommand {
    Add {
        triple: String,
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        toolchain: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum DevCommand {
    Stdlib {
        #[command(subcommand)]
        command: DevStdlibCommand,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum DevStdlibCommand {
    Package {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        sysroot: PathBuf,
        #[arg(long)]
        target: Option<String>,
        #[arg(long)]
        copy_source: bool,
        #[arg(long)]
        rockc: Option<PathBuf>,
    },
}
