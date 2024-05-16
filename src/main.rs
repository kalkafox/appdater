use std::path::Path;

use clap::{Parser, ValueEnum};
use futures_util::StreamExt;
use reqwest::header::{HeaderValue, USER_AGENT};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::{io::AsyncWriteExt, process::Command};
use url::Url;

#[derive(ValueEnum, Clone, Debug)]
enum AppSelection {
    VSCode,
    Discord,
}

pub type GitHubReleases = Vec<GitHubRelease>;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubRelease {
    pub url: String,
    #[serde(rename = "assets_url")]
    pub assets_url: String,
    #[serde(rename = "upload_url")]
    pub upload_url: String,
    #[serde(rename = "html_url")]
    pub html_url: String,
    pub id: i64,
    pub author: Author,
    #[serde(rename = "node_id")]
    pub node_id: String,
    #[serde(rename = "tag_name")]
    pub tag_name: String,
    #[serde(rename = "target_commitish")]
    pub target_commitish: String,
    pub name: String,
    pub draft: bool,
    pub prerelease: bool,
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(rename = "published_at")]
    pub published_at: String,
    pub assets: Vec<Value>,
    #[serde(rename = "tarball_url")]
    pub tarball_url: String,
    #[serde(rename = "zipball_url")]
    pub zipball_url: String,
    pub body: String,
    pub reactions: Reactions,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Author {
    pub login: String,
    pub id: i64,
    #[serde(rename = "node_id")]
    pub node_id: String,
    #[serde(rename = "avatar_url")]
    pub avatar_url: String,
    #[serde(rename = "gravatar_id")]
    pub gravatar_id: String,
    pub url: String,
    #[serde(rename = "html_url")]
    pub html_url: String,
    #[serde(rename = "followers_url")]
    pub followers_url: String,
    #[serde(rename = "following_url")]
    pub following_url: String,
    #[serde(rename = "gists_url")]
    pub gists_url: String,
    #[serde(rename = "starred_url")]
    pub starred_url: String,
    #[serde(rename = "subscriptions_url")]
    pub subscriptions_url: String,
    #[serde(rename = "organizations_url")]
    pub organizations_url: String,
    #[serde(rename = "repos_url")]
    pub repos_url: String,
    #[serde(rename = "events_url")]
    pub events_url: String,
    #[serde(rename = "received_events_url")]
    pub received_events_url: String,
    #[serde(rename = "type")]
    pub type_field: String,
    #[serde(rename = "site_admin")]
    pub site_admin: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reactions {
    pub url: String,
    #[serde(rename = "total_count")]
    pub total_count: i64,
    #[serde(rename = "+1")]
    pub n1: i64,
    #[serde(rename = "-1")]
    pub n12: i64,
    pub laugh: i64,
    pub hooray: i64,
    pub confused: i64,
    pub heart: i64,
    pub rocket: i64,
    pub eyes: i64,
}

/// Scrapes a site and checks bundler (js) size
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Directory to extract
    #[arg(long)]
    download_dir: Option<String>,

    /// Select app to download.
    #[arg(value_name = "app")]
    app: AppSelection,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    let should_run = true;

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
        download_dir = args
            .download_dir
            // this should never happen, but just incase
            .ok_or("Somehow, download dir was not found?")?;
    }

    match args.app {
        AppSelection::Discord => {
            download_discord(download_dir).await?;
        }
        AppSelection::VSCode => {
            download_vscode(download_dir).await?;
        }
    }

    println!("Done.");

    Ok(())
}

async fn download_vscode(download_dir: String) -> Result<(), Box<dyn std::error::Error>> {
    let vscode_dir = format!("{}/Apps/VSCode-linux-x64", download_dir);

    if !Path::new(&vscode_dir).exists() {
        println!("DiscordCanary directory not found.");
        std::process::exit(0)
    }

    let build_info_path = format!("{}/resources/app/package.json", vscode_dir);

    let build_info = std::fs::read_to_string(build_info_path)?;

    let build_info: serde_json::Value = serde_json::from_str(&build_info)?;

    let version_number = build_info["version"].as_str().unwrap();

    println!("VSCode version: {}", version_number);

    let mut headers = reqwest::header::HeaderMap::new();

    headers.append(USER_AGENT, HeaderValue::from_static("appdater 0.1.1"));

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .default_headers(headers)
        .build()?;

    let latest_data_url = "https://api.github.com/repos/microsoft/vscode/releases";

    let download_url = "https://code.visualstudio.com/sha/download?build=stable&os=linux-x64";

    let latest_data_res = client.get(latest_data_url).send().await?;

    let data = latest_data_res.json::<GitHubReleases>().await?;

    let latest_data = data.first().ok_or("No GitHub releases")?;

    println!("{:?}", latest_data.tag_name);

    let download_res = client.get(download_url).send().await?;

    if !download_res.status().is_redirection() {
        println!("Error downloading VSCode");
        std::process::exit(0)
    }

    // get the location
    let location = download_res
        .headers()
        .get("location")
        .ok_or("Location header not found")?;

    if version_number == latest_data.tag_name {
        println!("Version matches, no need to download");
        std::process::exit(0)
    }

    println!("Location: {:?}", location);

    let url = Url::parse(location.to_str()?)?;

    println!(
        "Downloading VSCode version {}, updating from {} into {}",
        latest_data.tag_name, version_number, download_dir
    );

    let res = client.get(url.as_str()).send().await?;

    if !res.status().is_success() {
        println!("Error downloading DiscordCanary");
        std::process::exit(0)
    }

    let mut file =
        tokio::fs::File::create(format!("/tmp/vscode-{}.tar.gz", latest_data.tag_name)).await?;

    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        file.write_all(&item?).await?;
    }

    let output = Command::new("tar")
        .arg("xzvf")
        .arg(format!("/tmp/vscode-{}.tar.gz", latest_data.tag_name))
        .arg("-C")
        .arg(format!("{}/Apps", download_dir))
        .output()
        .await?;

    if !output.status.success() {
        eprintln!("Error extracting tar file: {:?}", output);
    }

    Ok(())
}

async fn download_discord(download_dir: String) -> Result<(), Box<dyn std::error::Error>> {
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
    let location = res
        .headers()
        .get("location")
        .ok_or("Location header not found")?;

    println!("Location: {:?}", location);

    let url = Url::parse(location.to_str()?)?;

    let remote_version_number = url
        .path_segments()
        .ok_or("Path segments not found")?
        .nth(2)
        .ok_or("2nd element not found (expected /apps/linux/-<version_number>-)")?;

    println!("Version number: {}", version_number);

    if version_number == remote_version_number {
        println!("Version matches, no need to download");
        std::process::exit(0)
    }

    // std::process::exit(0);

    println!(
        "Downloading discord version {}, updating from {} into {}",
        remote_version_number, version_number, download_dir
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
        .arg(format!("{}/Downloads", download_dir))
        .output()
        .await?;

    if !output.status.success() {
        eprintln!("Error extracting tar file: {:?}", output);
    }

    Ok(())
}
