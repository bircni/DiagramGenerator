use anyhow::Context;
use clap::Parser as _;
use log::LevelFilter;
use simplelog::{ColorChoice, ConfigBuilder, TerminalMode};

mod logic;
mod svg;

#[cfg(test)]
mod tests;

#[derive(clap::Parser)]
#[command(author, version, about)]
/// Generate a diagram from Rust source code
pub struct Cli {
    /// Path to Cargo.toml
    #[clap(short, long)]
    pub path: std::path::PathBuf,
    /// Path to output the diagram
    #[clap(short, long, default_value = "output")]
    pub output_dir: std::path::PathBuf,
    /// Log Level Filter [Debug, Info, Error, Warn]
    #[clap(short, long, default_value = "Info")]
    pub loglevel: LevelFilter,
    /// Name of the Diagram
    #[clap(short, long, default_value = "Diagram")]
    pub name: String,
    /// Include test functions in the diagram (excluded by default)
    #[clap(short = 't', long, default_value = "false")]
    pub include_tests: bool,
}

fn initialize_logger(log_level: LevelFilter) -> anyhow::Result<()> {
    let level = if cfg!(debug_assertions) {
        LevelFilter::max()
    } else {
        log_level
    };

    simplelog::TermLogger::init(
        level,
        ConfigBuilder::new().build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .context("Failed to initialize logger")
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    initialize_logger(args.loglevel)?;

    let cargo_context = cargo::GlobalContext::default()?;

    let absolute_path =
        std::path::absolute(&args.path).context("Failed to resolve absolute path")?;
    let workspace = cargo::core::Workspace::new(&absolute_path, &cargo_context)
        .context("Failed to find workspace root")?;

    log::info!("Using workspace at {}", workspace.root().display());

    let package = workspace
        .load(&absolute_path)
        .context("Failed to load package")?;

    log::info!("Using package: {}", package.name());

    for target in package.targets() {
        log::info!("Found target: {}", target.name());

        let cargo::core::manifest::TargetSourcePath::Path(src) = target.src_path() else {
            anyhow::bail!("Target source path is not a file")
        };
        log::info!("Target source path: {}", src.display());

        let filename = format_output_filename(target.name(), target.kind());
        let output = args.output_dir.join(filename);
        std::fs::create_dir_all(&args.output_dir).context("Failed to create output directory")?;

        let mut visualizer = svg::SvgWriter::new(output);

        logic::ItemVisitor::visit_file(src, &mut visualizer, args.include_tests)
            .context("Failed to visit items in file")?;

        visualizer.finish()?;
    }

    Ok(())
}

pub(crate) fn format_output_filename(
    target_name: &str,
    target_kind: &cargo::core::manifest::TargetKind,
) -> String {
    use cargo::core::manifest::TargetKind;
    let kind_suffix = match target_kind {
        TargetKind::Lib(_) => "lib",
        TargetKind::Bin => "bin",
        TargetKind::Test => "test",
        TargetKind::Bench => "bench",
        TargetKind::ExampleLib(_) => "example-lib",
        TargetKind::ExampleBin => "example-bin",
        TargetKind::CustomBuild => "build",
    };
    format!("{target_name}-{kind_suffix}.svg")
}
