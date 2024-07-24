# Splat Tagger

Tag your [stat.ink](https://stat.ink) matches to a YouTube livestream with ease.

I noticed that matches on stat.ink has a "URL related to this battle" section. As I stream my Splatoon 3 games for archival purposes, I thought it would be nice to have a tool that automatically tags my games to the stream with their appropriate timestamp.

Thus, born this project.

```sh
$ splat-tagger --help
Tag your stat.ink matches to a YouTube livestream with ease!

Usage: splat-tagger [OPTIONS] <YOUTUBE_STREAM_URL>

Arguments:
  <YOUTUBE_STREAM_URL>  The Youtube stream URL to fetch data from (e.g. `dQw4w9WgXcQ`)

Options:
  -o, --offset <OFFSET>          Amount of seconds to adjust for any stream delays [default: 10]
      --config-dir <CONFIG_DIR>  The directory to store configuration files
  -n, --dry-run                  Whether to run the script without modifying any data
  -g, --gen-desc                 Generate a YouTube description containing a summary of the battles
  -h, --help                     Print help
  -V, --version                  Print version
```

## Installation

### via Manual Download

Download the latest release from the [releases page](https://github.com/angeloanan/splat-tagger/releases).

On Unix / macOS systems, you'll need to allow the binary to be executed with `chmod +x splat-tagger`.

Run the executable on a terminal (command prompt) with the YouTube stream URL as the first argument.

### via Cargo Install (Build from scratch)

Install Rust and Cargo from [rustup.rs](https://rustup.rs/) then run:

```sh
$ cargo install --git https://github.com/angeloanan/splat-tagger
```

## Configuration

The first run of the app will fail as the app lacks any configuration. The app will generate a configuration file in the your local app config directory:

-   **Windows**: `%LOCALAPPDATA%\splat-tagger`
-   **macOS**: `~/Library/Application Support/splat-tagger`
-   **Linux**: `$XDG_CONFIG_HOME/splat-tagger` or `~/.config/splat-tagger`

### Getting a Google API Key

To look up videos on YouTube, you'll need a Google API key.

1. Go to the [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project or select an existing one
3. Search for the [YouTube Data API v3](https://console.cloud.google.com/marketplace/product/google/youtube.googleapis.com) and enable the API
4. Go to the [APIs & Services > Credentials](https://console.cloud.google.com/apis/credentials) page and create a new credential (API key)
5. Copy the API key to the configuration file

### Getting stat.ink's Identity cookie

1. Log in to [stat.ink](https://stat.ink/)
2. Open the developer tools (F12) and go to the Application tab
3. On the left, go to the Cookies section and select `https://stat.ink`
4. Find a Cookie named `_identity` and copy its value to the configuration file

## Building

This project uses [Rust](https://rust-lang.org) and [Cargo](https://doc.rust-lang.org/cargo/)

To build the project, clone the repository and run `cargo build`.

## Contributing

Contributions are welcome, though not expected and not guaranteed to be merged; this is a personal project after all.

Feel free to fork and adapt this project to your needs.
