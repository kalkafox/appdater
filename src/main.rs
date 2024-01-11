use std::ops::{Add, Sub};

use clap::Parser;
use futures_util::StreamExt;
use tokio::{io::AsyncWriteExt, process::Command};

const LATEST_VERSION_NUMBER: i32 = 244;

/// Scrapes a site and checks bundler (js) size
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Directory to extract DiscordCanary
    #[arg(short, long)]
    download_dir: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    let should_run = true;

    #[cfg(not(target_os = "linux"))]
    let should_run = false;

    if !should_run {
        println!("Sorry, this can only run on Linux right now...");
        std::process::exit(0)
    }

    let args = Args::parse();

    let mut latest_version_number: i32 = LATEST_VERSION_NUMBER;
    let vars = std::env::vars();

    let home = vars
        .filter(|(key, _)| key == "HOME")
        .map(|(_, value)| value)
        .next();

    let mut download_dir = home.ok_or("HOME environment variable not found")?;

    if args.download_dir.is_some() {
        download_dir = args.download_dir.unwrap();
    }

    let url = &format!(
        "https://dl-canary.discordapp.net/apps/linux/0.0.{}/discord-canary-0.0.{}.tar.gz",
        LATEST_VERSION_NUMBER, LATEST_VERSION_NUMBER
    );

    let res = reqwest::get(url).await?;

    let mut status = res.status();

    while status.as_u16() == 200 {
        latest_version_number = latest_version_number.add(1);

        let res = reqwest::get(url.replace(
            &LATEST_VERSION_NUMBER.to_string(),
            &latest_version_number.to_string(),
        ))
        .await?;

        status = res.status();
    }

    latest_version_number = latest_version_number.sub(1);

    let res = reqwest::get(url.replace(
        &LATEST_VERSION_NUMBER.to_string(),
        &latest_version_number.to_string(),
    ))
    .await?;

    println!("Downloading discord version {}", latest_version_number);

    let mut file = tokio::fs::File::create(format!(
        "/tmp/discord-canary-0.0.{}.tar.gz",
        latest_version_number
    ))
    .await?;

    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        file.write_all(&item?).await?;
    }

    let output = Command::new("tar")
        .arg("xzvf")
        .arg(format!(
            "/tmp/discord-canary-0.0.{}.tar.gz",
            latest_version_number
        ))
        .arg("-C")
        .arg(format!("{}/Downloads", download_dir))
        .output()
        .await?;

    if !output.status.success() {
        eprintln!("Error extracting tar file: {:?}", output);
    }

    println!("Done.");

    Ok(())
}
