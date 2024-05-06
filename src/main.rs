#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::cargo)]
#![warn(clippy::perf)]
#![warn(clippy::complexity)]
#![warn(clippy::style)]

mod battle;
mod config;
mod salmon;
mod youtube;

use chrono::DateTime;
use config::Config;
use std::{path::PathBuf, process::exit, sync::Arc};
use tokio::task::JoinSet;

use clap::{command, Parser};
use tracing::{error, info, trace, warn};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The Youtube stream URL to fetch data from (e.g. `dQw4w9WgXcQ`)
    youtube_stream_url: String,

    // /// Whether to modify the most recent Battle data (incl. Ranked & Splatfest)
    // #[arg(short, long, default_value_t = true)]
    // battle: bool,

    // /// Whether to modify the most recent Salmon Run data
    // #[arg(short, long, default_value_t = true)]
    // salmon_run: bool,
    /// Amount of seconds to adjust for any stream delays
    #[arg(short, long, default_value_t = 0)]
    offset: i64,

    /// The directory to store configuration files
    #[arg(long)]
    config_dir: Option<PathBuf>,
}

#[allow(clippy::too_many_lines)]
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    trace!("Arguments passed successfully");

    let config_dir = args.config_dir.unwrap_or_else(|| {
        trace!("No configuration directory specified, using default");
        dirs::config_dir()
            .expect("Could not find a default configuration directory")
            .join(env!("CARGO_PKG_NAME"))
    });

    // Create config directory if it doesn't exist
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir).expect("Could not create configuration directory!");
        std::fs::write(
            config_dir.join("config.toml"),
            toml::to_string(&Config::default()).unwrap().as_bytes(),
        )
        .unwrap();
        error!("No configuration files found!");
        panic!(
            "Please fill in your config file at {}!",
            config_dir.display()
        );
    }

    let config = toml::from_str::<Config>(
        &std::fs::read_to_string(config_dir.join("config.toml"))
            .expect("Could not read configuration file!"),
    )
    .unwrap_or_else(|_| Config::default());

    info!(
        "Using config file: {}",
        config_dir.join("config.toml").display()
    );
    info!("Livestream ID: {}", args.youtube_stream_url);
    info!("Stat.ink Username: {}", config.statink.username);

    let http_cookie_jar = reqwest::cookie::Jar::default();
    http_cookie_jar.add_cookie_str(
        &format!(
            "_identity={}; Domain=stat.ink",
            config.statink.identity_cookie
        ),
        &"https://stat.ink".parse::<reqwest::Url>().unwrap(),
    );
    let client = reqwest::Client::builder()
        .cookie_provider(Arc::new(http_cookie_jar))
        .user_agent(format!(
            "Mozilla/5.0 (compatible; {}/{}; +{})",
            env!("CARGO_BIN_NAME"),
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_PKG_REPOSITORY")
        ))
        .build()
        .expect("Unable to create HTTP client!");

    let (livestream_data, recent_salmon_data, recent_battle_data) = tokio::join!(
        youtube::fetch_video_data(
            client.clone(),
            &config.google_api_key,
            &args.youtube_stream_url
        ),
        salmon::get_salmon_log(client.clone(), &config.statink.username),
        battle::get_battle_log(client.clone(), &config.statink.username)
    );

    let stream_start_time =
        DateTime::parse_from_rfc3339(&livestream_data.live_streaming_details.actual_start_time)
            .unwrap();
    let stream_end_time =
        DateTime::parse_from_rfc3339(&livestream_data.live_streaming_details.actual_end_time)
            .unwrap();

    info!("");
    info!("Stream start: {}", stream_start_time);
    info!("Stream end: {}", stream_end_time);

    let salmon_runs_to_modify: salmon::LogList = recent_salmon_data
        .iter()
        .filter(|&d| {
            let run_start_time = DateTime::parse_from_rfc3339(&d.start_at.iso8601).unwrap();
            run_start_time > stream_start_time && run_start_time < stream_end_time
        })
        .cloned()
        .collect();
    info!(
        "Found {} salmon runs to modify",
        salmon_runs_to_modify.len()
    );

    let battle_to_modify: battle::LogList = recent_battle_data
        .iter()
        .filter(|&d| {
            let battle_start_time = DateTime::parse_from_rfc3339(&d.start_at.iso8601).unwrap();
            battle_start_time > stream_start_time && battle_start_time < stream_end_time
        })
        .cloned()
        .collect();
    info!("Found {} battles to modify", battle_to_modify.len());

    assert!(
        !(!salmon_runs_to_modify.is_empty() && !battle_to_modify.is_empty()),
        "Found no battles to modify!"
    );

    let mut data_mod: JoinSet<()> = JoinSet::new();
    for run in salmon_runs_to_modify {
        let run_start_time = DateTime::parse_from_rfc3339(&run.start_at.iso8601).unwrap();
        let difference = run_start_time - stream_start_time;
        let link = format!(
            "https://youtube.com/watch?v={}&t={}s",
            args.youtube_stream_url,
            difference.num_seconds() + args.offset
        );

        data_mod.spawn(salmon::add_link_to_salmon_log(
            client.clone(),
            run.id.leak(),
            link.leak(),
        ));
    }
    for battle in battle_to_modify {
        let battle_start_time = DateTime::parse_from_rfc3339(&battle.start_at.iso8601).unwrap();
        let difference = battle_start_time - stream_start_time;
        let link = format!(
            "https://youtube.com/watch?v={}&t={}s",
            args.youtube_stream_url,
            difference.num_seconds() + args.offset
        );

        data_mod.spawn(battle::add_link_to_battle_log(
            client.clone(),
            battle.id.leak(),
            link.leak(),
        ));
    }

    while let Some(fut) = data_mod.join_next().await {
        if let Err(e) = fut {
            error!("Unable to update a run. Is your Identity cookie expired?");
            error!("Please double check your configuration file and try again.");
            error!("Error: {}", e);
            exit(1);
        }
    }

    info!("Finished modifying all runs!");
}
