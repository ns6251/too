use std::borrow::Cow;
use std::path::PathBuf;

use clap::{ArgEnum, Parser};
use futures::future::try_join_all;
use regex::Regex;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
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

    let mut buf = String::new();
    let mut stdin = tokio::io::stdin();
    stdin.read_to_string(&mut buf).await?;

    let mut stdout = tokio::io::stdout();
    let stdout = stdout.write(buf.as_bytes());

    let plain = decolorize(&buf);

    let mut option = OpenOptions::new();
    option.create(true).write(true).append(cli.append).truncate(!cli.append);
    let files = cli.file.iter().map(|file| option.open(file));
    let mut files = try_join_all(files).await?;

    let files = try_join_all(files.iter_mut().map(|file| file.write(plain.as_bytes())));

    tokio::try_join!(stdout, files)?;

    Ok(())
}
