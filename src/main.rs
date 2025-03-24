use anyhow::Context;
use clap::Parser as _;
use cli::Cli;
use log::{LevelFilter, log_enabled};
use serde_json::json;
use simplelog::{ColorChoice, ConfigBuilder, TerminalMode};
use std::{env, fs, path::PathBuf, process};
use tinytemplate::TinyTemplate;

mod cli;
mod items;
mod logic;

const HTML_TEMPLATE: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>{title}</title>
    <link rel="stylesheet" href="diagram.css">
</head>
<style>{ contents | style }</style>
<body>
    <h1>{title}</h1>

    {contents}
</body>
</html>
"#;

fn main() {
    if let Err(e) = real_main() {
        log::error!("{e:#}");
        process::exit(1);
    }
}

fn real_main() -> anyhow::Result<()> {
    let args = Cli::parse_from(env::args().filter(|a| a != "diagram"));
    initialize_logger(args.loglevel)?;
    let path = find_path(args.path)?;
    log::debug!("Using file: {}", path.display());

    let contents =
        logic::parse_file_recursive(&path).context(format!("Failed to parse file: {path:?}"))?;

    let mut tt = TinyTemplate::new();
    tt.set_default_formatter(&tinytemplate::format_unescaped);
    tt.add_formatter("style", |_, string| {
        string.push_str(include_str!("style.css"));
        Ok(())
    });
    tt.add_template("html", HTML_TEMPLATE)
        .context("Failed to add template")?;
    let html = tt
        .render("html", &json!({"title": args.name, "contents": contents}))
        .context("Failed to render template")?;

    let out_path = if let Some(path) = args.output {
        if !path.exists() {
            fs::create_dir_all(&path).context("Failed to create directory")?;
        }
        path
    } else {
        PathBuf::from("diagram.html")
    };

    fs::write(&out_path, html).context("Failed to write file")?;
    log::info!("Wrote the output to {}", out_path.display());

    Ok(())
}

/// Returns the provided path or searches for main.rs or lib.rs in the current directory
fn find_path(provided_path: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    let file_path = if let Some(path) = provided_path {
        path
    } else {
        log::info!(
            "No input file provided, searching for main.rs or lib.rs in the current directory"
        );
        let mut current_dir = env::current_dir().context("Failed to get current directory")?;
        if current_dir.join("Cargo.toml").exists() {
            log::info!("Cargo.toml found, assuming a Cargo project");
            current_dir = current_dir.join("src");
        }
        // find main.rs or lib.rs in the current directory
        current_dir
            .read_dir()
            .context("Failed to read directory")?
            .find(|entry| {
                entry
                    .as_ref()
                    .map(|file_entry| {
                        file_entry
                            .file_name()
                            .to_string_lossy()
                            .to_string()
                            .ends_with("main.rs")
                            || file_entry
                                .file_name()
                                .to_string_lossy()
                                .to_string()
                                .ends_with("lib.rs")
                    })
                    .unwrap_or(false)
            })
            .context(format!(
                "Failed to find main.rs or lib.rs in {}",
                current_dir.display()
            ))?
            .context("Failed to get directory entry")?
            .path()
    };

    Ok(file_path)
}

/// Initialize the logger
/// # Errors
/// Fails if the logger could not be initialized
fn initialize_logger(log_level: LevelFilter) -> anyhow::Result<()> {
    let filter = if cfg!(debug_assertions) {
        LevelFilter::max()
    } else {
        log_level
    };
    if !log_enabled!(filter.to_level().context("Failed to get log level")?) {
        return simplelog::TermLogger::init(
            filter,
            ConfigBuilder::new()
                // suppress all logs from dependencies
                .add_filter_allow_str("cargo_diagram")
                .build(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        )
        .context("Failed to initialize logger");
    }
    Ok(())
}
