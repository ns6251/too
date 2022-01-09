use std::borrow::Cow;
use std::path::{Path, PathBuf};

use clap::{ArgEnum, Parser};
use futures::future::try_join_all;
use regex::Regex;
use tokio::fs::OpenOptions;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::signal;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Cli {
    /// Append to the given FILEs, do not overwrite
    #[clap(short, long)]
    pub append: bool,

    /// Ignore interrupt signals
    #[clap(short, long)]
    pub ignore_interrupts: bool,

    /// Diagnose errors writing to non pipes
    #[clap(short)]
    pub p: bool,

    /// Set behavior on write error
    #[clap(long, arg_enum, default_value_t = Mode::WarnNopipe)]
    pub output_error: Mode,

    pub file: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
enum Mode {
    /// Diagnose errors writing to any output
    Warn,

    /// Diagnose errors writing to any output not a pipe
    WarnNopipe,

    /// Exit on error writing to any output
    Exit,

    /// Exit on error writing to any output not a pipe
    ExitNopipe,
}

/// Remove ANSI escape sequences
fn decolorize(s: &str) -> Cow<str> {
    let re = Regex::new(r#"\x1B\[([0-9]{1,3}(;[0-9]{1,2})?)?[mGK]"#).unwrap();
    re.replace_all(s, "")
}

async fn read_from_stdin() -> io::Result<String> {
    let mut buf = String::new();
    let mut stdin = tokio::io::stdin();
    stdin.read_to_string(&mut buf).await?;
    return Ok(buf);
}

async fn write_to_stdout(s: &str) -> io::Result<()> {
    let mut stdout = tokio::io::stdout();
    stdout.write(s.as_bytes()).await?;
    Ok(())
}

async fn write_to_file(path: &Path, s: &str, append: bool) -> io::Result<()> {
    let mut option = OpenOptions::new();
    option
        .create(true)
        .write(true)
        .append(append)
        .truncate(!append)
        .open(path)
        .await?
        .write(s.as_bytes())
        .await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.ignore_interrupts {
        tokio::spawn(async {
            loop {
                signal::ctrl_c().await.unwrap();
            }
        });
    }

    let input = read_from_stdin().await?;

    let plain = decolorize(&input);

    let files = cli
        .file
        .iter()
        .map(|file| write_to_file(file, plain.as_ref(), cli.append));
    let files = try_join_all(files);

    tokio::try_join!(write_to_stdout(&input), files)?;

    Ok(())
}
