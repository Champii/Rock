use std::path::PathBuf;

use clap::Parser;

use crate::cli::{
    CommandConfig, Config, DevCommand, DevStdlibCommand, TargetCommand, ToolchainCommand,
};

#[test]
fn test_cli_parses_install_command() {
    let config = Config::try_parse_from([
        "rockup",
        "toolchain",
        "install",
        "stable",
        "--path",
        "/tmp/toolchain",
    ])
    .unwrap();

    match config.command {
        CommandConfig::Toolchain {
            command: ToolchainCommand::Install { name, path },
        } => {
            assert_eq!(name, "stable");
            assert_eq!(path, PathBuf::from("/tmp/toolchain"));
        }
        other => panic!("unexpected command: {:?}", other),
    }
}

#[test]
fn test_cli_parses_remove_command() {
    let config = Config::try_parse_from(["rockup", "toolchain", "remove", "stable"]).unwrap();

    match config.command {
        CommandConfig::Toolchain {
            command: ToolchainCommand::Remove { name },
        } => {
            assert_eq!(name, "stable");
        }
        other => panic!("unexpected command: {:?}", other),
    }
}

#[test]
fn test_cli_parses_target_add_command() {
    let config = Config::try_parse_from([
        "rockup",
        "target",
        "add",
        "wasm32-unknown-unknown",
        "--path",
        "/tmp/component",
        "--toolchain",
        "stable",
    ])
    .unwrap();

    match config.command {
        CommandConfig::Target {
            command:
                TargetCommand::Add {
                    triple,
                    path,
                    toolchain,
                },
        } => {
            assert_eq!(triple, "wasm32-unknown-unknown");
            assert_eq!(path, PathBuf::from("/tmp/component"));
            assert_eq!(toolchain.as_deref(), Some("stable"));
        }
        other => panic!("unexpected command: {:?}", other),
    }
}

#[test]
fn test_cli_parses_env_command() {
    let config = Config::try_parse_from(["rockup", "env"]).unwrap();

    match config.command {
        CommandConfig::Env => {}
        other => panic!("unexpected command: {:?}", other),
    }
}

#[test]
fn test_cli_parses_dev_stdlib_package_command() {
    let config = Config::try_parse_from([
        "rockup",
        "dev",
        "stdlib",
        "package",
        "--path",
        "/tmp/stdlib",
        "--sysroot",
        "/tmp/dev-sysroot",
        "--target",
        "wasm32-unknown-unknown",
        "--copy-source",
        "--rockc",
        "/tmp/rockc",
    ])
    .unwrap();

    match config.command {
        CommandConfig::Dev {
            command:
                DevCommand::Stdlib {
                    command:
                        DevStdlibCommand::Package {
                            path,
                            sysroot,
                            target,
                            copy_source,
                            rockc,
                        },
                },
        } => {
            assert_eq!(path, PathBuf::from("/tmp/stdlib"));
            assert_eq!(sysroot, PathBuf::from("/tmp/dev-sysroot"));
            assert_eq!(target.as_deref(), Some("wasm32-unknown-unknown"));
            assert!(copy_source);
            assert_eq!(rockc.as_deref(), Some(std::path::Path::new("/tmp/rockc")));
        }
        other => panic!("unexpected command: {:?}", other),
    }
}
