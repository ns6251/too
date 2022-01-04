use clap::Parser;
use futures::future::try_join_all;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Cli {
    file: Vec<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let mut buf = String::new();
    let mut stdin = tokio::io::stdin();
    stdin.read_to_string(&mut buf).await?;

    let mut stdout = tokio::io::stdout();
    let stdout = stdout.write(buf.as_bytes());

    let files = cli.file.iter().map(|path| tokio::fs::write(path, &buf));
    let files = try_join_all(files);

    tokio::try_join!(stdout, files)?;

    Ok(())
}
