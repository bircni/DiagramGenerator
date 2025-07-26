use anyhow::Context;
use clap::Parser as _;
use cli::Cli;
use log::{LevelFilter, log_enabled};
use serde_json::json;
use simplelog::{ColorChoice, ConfigBuilder, TerminalMode};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process,
};
use tinytemplate::TinyTemplate;

/// Pipe operator for functional-style programming
trait Pipe<T> {
    fn pipe<U, F>(self, f: F) -> U
    where
        F: FnOnce(T) -> U;
}

impl<T> Pipe<T> for T {
    fn pipe<U, F>(self, f: F) -> U
    where
        F: FnOnce(T) -> U,
    {
        f(self)
    }
}

mod cli;
mod items;
mod logic;

const HTML_TEMPLATE: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>{title}</title>
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <link rel="stylesheet" href="diagram.css">
</head>
<style>{style_contents}</style>
<body>
    <header>
        <h1>{title}</h1>
        <div class="controls">
            <p class="instruction">Click module names to collapse/expand</p>
        </div>
    </header>

    <main id="content">
        {contents}
    </main>
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

    let contents = logic::parse_file_recursive(&path, args.include_tests)
        .context(format!("Failed to parse file: {}", path.display()))?;

    let mut tt = TinyTemplate::new();
    tt.set_default_formatter(&tinytemplate::format_unescaped);
    tt.add_template("html", HTML_TEMPLATE)
        .context("Failed to add template")?;
    let html = tt
        .render(
            "html",
            &json!({
                "title": args.name,
                "contents": contents,
                "style_contents": include_str!("style.css")
            }),
        )
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
/// If provided path is a repository root, automatically finds the entry point
fn find_path(provided_path: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    let file_path = if let Some(path) = provided_path {
        // Check if the provided path is a directory (repository root)
        if path.is_dir() {
            log::info!(
                "Directory provided: {}, searching for entry point",
                path.display()
            );
            find_entry_point_in_directory(&path)?
        } else {
            // It's already a file path
            path
        }
    } else {
        log::info!(
            "No input file provided, searching for main.rs or lib.rs in the current directory"
        );
        let current_dir = env::current_dir().context("Failed to get current directory")?;
        find_entry_point_in_directory(&current_dir)?
    };

    Ok(file_path)
}

/// Find the main entry point (main.rs or lib.rs) in a given directory
fn find_entry_point_in_directory(dir: &Path) -> anyhow::Result<PathBuf> {
    let mut search_dir = dir.to_path_buf();

    // Check if this is a Cargo project root
    if search_dir.join("Cargo.toml").exists() {
        log::info!(
            "Cargo.toml found in {}, assuming a Cargo project",
            search_dir.display()
        );
        search_dir = search_dir.join("src");

        // First try to find lib.rs (library crate)
        let lib_path = search_dir.join("lib.rs");
        if lib_path.exists() {
            log::info!("Found lib.rs at {}", lib_path.display());
            return Ok(lib_path);
        }

        // Then try main.rs (binary crate)
        let main_path = search_dir.join("main.rs");
        if main_path.exists() {
            log::info!("Found main.rs at {}", main_path.display());
            return Ok(main_path);
        }
    }

    // If not a Cargo project or no lib.rs/main.rs in src/, search in the directory itself
    search_dir
        .read_dir()
        .context(format!("Failed to read directory {}", search_dir.display()))?
        .find(|entry| {
            entry
                .as_ref()
                .map(|file_entry| {
                    let file_name = file_entry.file_name().to_string_lossy().to_string();
                    file_name == "lib.rs" || file_name == "main.rs"
                })
                .unwrap_or(false)
        })
        .context(format!(
            "Failed to find main.rs or lib.rs in {}",
            search_dir.display()
        ))?
        .context("Failed to get directory entry")?
        .path()
        .pipe(Ok)
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
