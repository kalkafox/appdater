use std::path::Path;

use clap::Parser;
use futures_util::StreamExt;
use tokio::{io::AsyncWriteExt, process::Command};
use url::Url;

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
    let vars = std::env::vars();

    let home = vars
        .filter(|(key, _)| key == "HOME")
        .map(|(_, value)| value)
        .next();

    let mut download_dir = home.ok_or("HOME environment variable not found")?;

    if args.download_dir.is_some() {
        download_dir = args.download_dir.unwrap();
    }

    // Check if args.download_dir/Downloads/DiscordCanary exists
    let discord_dir = format!("{}/Downloads/DiscordCanary", download_dir);

    if !Path::new(&discord_dir).exists() {
        println!("DiscordCanary directory not found.");
        std::process::exit(0)
    }

    // Read the version number from the latest DiscordCanary file (located in args.download_dir/Downloads/DiscordCanary/resources/build_info.json)
    let build_info_path = format!("{}/resources/build_info.json", discord_dir);

    let build_info = std::fs::read_to_string(build_info_path)?;

    let build_info: serde_json::Value = serde_json::from_str(&build_info)?;

    let version_number = build_info["version"].as_str().unwrap();

    println!("DiscordCanary version: {}", version_number);

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    let download_url = "https://discordapp.com/api/download/canary?platform=linux&format=tar.gz";

    let res = client.get(download_url).send().await?;

    if !res.status().is_redirection() {
        println!("Error downloading DiscordCanary");
        std::process::exit(0)
    }

    // get the location
    let location = res.headers().get("location").unwrap();

    println!("Location: {:?}", location);

    let url = Url::parse(location.to_str()?)?;

    let remote_version_number = url.path_segments().unwrap().nth(2).unwrap();

    println!("Version number: {}", version_number);

    if version_number == remote_version_number {
        println!("Version matches, no need to download");
        std::process::exit(0)
    }

    // std::process::exit(0);

    println!(
        "Downloading discord version {}, updating from {}",
        remote_version_number, version_number
    );

    let res = client.get(url.as_str()).send().await?;

    if !res.status().is_success() {
        println!("Error downloading DiscordCanary");
        std::process::exit(0)
    }

    let mut file = tokio::fs::File::create(format!(
        "/tmp/discord-canary-0.0.{}.tar.gz",
        remote_version_number
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
            remote_version_number
        ))
        .arg("-C")
        .arg(format!("{}", download_dir))
        .output()
        .await?;

    if !output.status.success() {
        eprintln!("Error extracting tar file: {:?}", output);
    }

    println!("Done.");

    Ok(())
}
