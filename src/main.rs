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

use chrono::{DateTime, Duration};
use config::Config;
use copypasta::ClipboardProvider;
use salmon::tide_to_abbr;
use std::{collections::BTreeMap, path::PathBuf, process::exit, sync::Arc};
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
    #[arg(short, long, default_value_t = 10)]
    offset: i64,

    /// The directory to store configuration files
    #[arg(long)]
    config_dir: Option<PathBuf>,

    /// Whether to run the script without modifying any data
    #[arg(short = 'n', long = "dry-run", default_value_t = false)]
    dry_run: bool,

    /// Generate a YouTube description containing a summary of the battles
    #[arg(short = 'g', long = "gen-desc", default_value_t = false)]
    generate_description: bool,
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

    // Parse YouTube URL to get the video ID
    let livestream_id = if args.youtube_stream_url.contains("youtube.com/watch") {
        args.youtube_stream_url.split('=').last().map_or_else(
            || {
                error!("Invalid YouTube URL provided!");
                exit(1);
            },
            std::string::ToString::to_string,
        )
    } else if args.youtube_stream_url.contains("youtube.com/live") {
        args.youtube_stream_url.split('/').last().map_or_else(
            || {
                error!("Invalid YouTube URL provided!");
                exit(1);
            },
            std::string::ToString::to_string,
        )
    } else {
        args.youtube_stream_url
    };

    info!("Livestream ID: {}", livestream_id);
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
        youtube::fetch_video_data(client.clone(), &config.google_api_key, &livestream_id),
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

    let battle_to_modify: battle::LogList = recent_battle_data
        .iter()
        .filter(|&d| {
            let battle_start_time = DateTime::parse_from_rfc3339(&d.start_at.iso8601).unwrap();
            battle_start_time > stream_start_time && battle_start_time < stream_end_time
        })
        .cloned()
        .collect();
    info!("Found {} battles to modify", battle_to_modify.len());

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

    assert!(
        !(battle_to_modify.is_empty() && salmon_runs_to_modify.is_empty()),
        "Found no battles to modify!"
    );

    let mut data_mod: JoinSet<()> = JoinSet::new();
    for run in &salmon_runs_to_modify {
        let run_start_time = DateTime::parse_from_rfc3339(&run.start_at.iso8601).unwrap();
        let difference = run_start_time - stream_start_time;
        let link = format!(
            "https://youtube.com/watch?v={}&t={}s",
            &livestream_id,
            difference.num_seconds() + args.offset
        );

        if args.dry_run {
            info!("Will modify run {} with URL {link}", run.id);
        } else {
            data_mod.spawn(salmon::add_link_to_salmon_log(
                client.clone(),
                run.id.clone(),
                link,
            ));
        }
    }
    for battle in &battle_to_modify {
        let battle_start_time = DateTime::parse_from_rfc3339(&battle.start_at.iso8601).unwrap();
        let difference = battle_start_time - stream_start_time;
        let link = format!(
            "https://youtube.com/watch?v={}&t={}s",
            &livestream_id,
            difference.num_seconds() + args.offset
        );

        if args.dry_run {
            info!("Will modify battle {} with URL {link}", battle.id);
        } else {
            data_mod.spawn(battle::add_link_to_battle_log(
                client.clone(),
                battle.id.clone(),
                link,
            ));
        }
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

    if args.generate_description {
        info!("Generating YouTube description...");

        let mut timestamps: BTreeMap<i64, String> = BTreeMap::new();
        timestamps.insert(0, "00:00:00 Warming up...".to_string());

        // 00:00:00 Turf War Win (50.0% vs 49.9%)
        // 00:00:00 Tricolor Win (33.3% vs 21.9% vs 11.1%)
        // 00:00:00 Rainmaker Open Win (C+ 1980)
        // 00:00:00 Zones Open Win (C+ 1980)
        // 00:00:00 Clam Open Win (100pts vs 5pts)
        // 00:00:00 Tower Open Win ()

        // TODO: Finish this
        // for battle in battle_to_modify {
        //     let battle_start_time = DateTime::parse_from_rfc3339(&battle.start_at.iso8601).unwrap();
        //     let difference = battle_start_time - stream_start_time + Duration::seconds(args.offset);
        //     let mut line = format!(
        //         "{:02}:{:02}:{:02} {} ",
        //         difference.num_hours(),
        //         difference.num_minutes() % 60,
        //         difference.num_seconds() % 60,
        //     );

        //     timestamps.insert(difference.num_seconds(), line);
        // }

        for runs in salmon_runs_to_modify {
            let run_start_time = DateTime::parse_from_rfc3339(&runs.start_at.iso8601).unwrap();
            let difference = run_start_time - stream_start_time + Duration::seconds(args.offset);
            let mut line = format!(
                "{:02}:{:02}:{:02} SR {}% {} (",
                difference.num_hours(),
                difference.num_minutes() % 60,
                difference.num_seconds() % 60,
                runs.danger_rate.unwrap_or(0),
                runs.golden_eggs,
            );

            let mut wave_num = 0;
            for wave in &runs.waves {
                wave_num += 1;
                if wave_num > 3 {
                    continue;
                }

                // Deliveries
                let dv = wave.golden_delivered;
                let tide = tide_to_abbr(&wave.tide.key);
                if let Some(event) = &wave.event {
                    match event.key.as_str() {
                        "giant_tornado" => line.push_str(&format!("Tornado {dv}")),
                        "rush" => line.push_str(&format!("Rush {dv}")),
                        "cohock_charge" => line.push_str(&format!("Cohock {dv}")),
                        "mothership" => line.push_str(&format!("Mothership {dv}")),
                        "griller" => line.push_str(&format!("Griller {dv}")),
                        "fog" => line.push_str(&format!("{tide} Fog {dv}")),
                        "goldie_seeking" => line.push_str(&format!("{tide} Seeking {dv}")),
                        "mudmouth_eruption" => line.push_str(&format!("{tide} Mudmouth {dv}")),
                        _ => line.push_str("UNKNOWN EVENT"),
                    };
                } else {
                    line.push_str(&format!("{tide} {dv}"));
                }

                if wave_num < 3 && runs.waves.len() != wave_num {
                    line.push_str(", ");
                }
            }

            line.push(')');
            timestamps.insert(difference.num_seconds(), line);
        }

        println!();
        for stamps in &timestamps {
            println!("{}", stamps.1);
        }
        println!();

        let mut ctx =
            copypasta::ClipboardContext::new().expect("Unable to create clipboard context");
        ctx.set_contents(
            timestamps
                .iter()
                .map(|t| t.1.to_string())
                .collect::<Vec<String>>()
                .join("\n"),
        )
        .expect("Unable to copy timestamps to clipboard!");

        info!("The YouTube description has been copied to your clipboard!");
    }
}
