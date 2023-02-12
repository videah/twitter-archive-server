use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct ScrapedTweet {
    pub full_text: String,
    pub user: ScrapedUser,
    pub extended_entities: Option<ScrapedExtendedEntities>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ScrapedUser {
    pub screen_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ScrapedEntities {
    pub media: Vec<ScrapedMedia>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ScrapedExtendedEntities {
    pub media: Vec<ScrapedExtendedMedia>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ScrapedExtendedMedia {
    #[serde(rename = "type")]
    pub media_type: String,
    pub video_info: Option<ScrapedVideoInfo>,
    pub media_url_https: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ScrapedVideoInfo {
    pub variants: Vec<ScrapedVariant>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ScrapedVariant {
    pub bitrate: Option<u32>,
    pub content_type: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ScrapedMedia {
    pub media_url_https: String,
}