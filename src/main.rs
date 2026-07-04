mod codegen;
mod frontend;

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

#[derive(Parser)]
#[command(name = "ncc", version, about = "Niche C Compiler")]
struct Cli {
    /// C source file to compile.
    input: PathBuf,

    /// Output object file.
    #[arg(short, long, default_value = "a.o")]
    output: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let source = fs::read_to_string(&cli.input)
        .with_context(|| format!("failed to read {}", cli.input.display()))?;
    let function = frontend::parse(&source)
        .with_context(|| format!("failed to compile {}", cli.input.display()))?;
    let object = codegen::emit_object(&function)?;
    fs::write(&cli.output, object)
        .with_context(|| format!("failed to write {}", cli.output.display()))?;
    Ok(())
}
