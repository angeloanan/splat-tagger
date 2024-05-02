use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument};

#[instrument(skip(http_client))]
pub async fn get_battle_log(http_client: Client, username: &str) -> LogList {
    let statink_battle_api_url =
        Url::parse(&format!("https://stat.ink/@{username}/spl3/index.json"))
            .expect("Unable to create stat.ink Splat URL");
    debug!("Fetching Battle log ({statink_battle_api_url})");

    let battle_log_request = http_client
        .get(statink_battle_api_url)
        .send()
        .await
        .expect("Unable to fetch Battle Log");
    debug!("Transforming Battle logs to JSON...");

    let out = battle_log_request
        .json::<LogList>()
        .await
        .expect("Unable to parse Battle logs");
    debug!("Done!");

    out
}

#[instrument(skip(http_client, link))]
pub async fn add_link_to_battle_log(http_client: Client, log_uuid: &str, link: &str) {
    let endpoint_url = Url::parse_with_params(
        "https://stat.ink/api/internal/patch-battle3-url",
        &[("id", log_uuid)],
    )
    .unwrap();

    debug!("Updating Battle log");
    let update_request = http_client
        .post(endpoint_url)
        .form(&[("_method", "PATCH"), ("link_url", link)])
        .send()
        .await;

    update_request.expect("Unable to update Battle log!");
    info!("Updated Battle log!");
}

// Code below generated by https://quicktype.io
// Example code that deserializes and serializes the model.
// extern crate serde;
// #[macro_use]
// extern crate serde_derive;
// extern crate serde_json;
//
// use generated_module::BattleList;
//
// fn main() {
//     let json = r#"{"answer": 42}"#;
//     let model: BattleList = serde_json::from_str(&json).unwrap();
// }

pub type LogList = Vec<Log>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log {
    pub id: String,
    pub url: String,
    pub uuid: String,
    pub start_at: Time,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Time {
    pub time: i64,
    pub iso8601: String,
}
