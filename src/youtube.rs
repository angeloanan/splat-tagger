use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

pub const YOUTUBE_VIDEO_DATA_API_URL: &str = "https://www.googleapis.com/youtube/v3/videos";

#[instrument(skip(http_client, api_key))]
pub async fn fetch_video_data(http_client: Client, api_key: &str, video_id: &str) -> Item {
    debug!("Fetching data");
    let livestream_query_url = Url::parse_with_params(
        YOUTUBE_VIDEO_DATA_API_URL,
        &[
            ("part", "liveStreamingDetails"),
            ("id", video_id),
            ("key", api_key),
        ],
    )
    .expect("Unable to create URL");

    let livestream_request = http_client
        .get(livestream_query_url)
        .send()
        .await
        .expect("Unable to fetch YouTube livestream data. Are you connected to the internet?");
    debug!("Parsing data to JSON...");

    let livestream_data = livestream_request
        .json::<YouTubeDataList>()
        .await
        .expect("Unable to parse JSON response");

    let livestream_items = livestream_data.items;
    debug!("Livestream data: {livestream_items:?}");
    assert!(
        !livestream_items.is_empty(),
        "Livestream ID not found! Did you copy the correct ID?"
    );

    livestream_items.first().unwrap().to_owned()
}

// Code below generated by https://quicktype.io
// Example code that deserializes and serializes the model.
// extern crate serde;
// #[macro_use]
// extern crate serde_derive;
// extern crate serde_json;
//
// use generated_module::YouTubeDataList;
//
// fn main() {
//     let json = r#"{"answer": 42}"#;
//     let model: YouTubeDataList = serde_json::from_str(&json).unwrap();
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YouTubeDataList {
    pub kind: String,
    pub etag: String,
    pub items: Vec<Item>,
    pub page_info: PageInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub kind: String,
    pub etag: String,
    pub id: String,
    pub live_streaming_details: LiveStreamingDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveStreamingDetails {
    pub actual_start_time: String,
    pub actual_end_time: String,
    pub scheduled_start_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub total_results: i64,
    pub results_per_page: i64,
}