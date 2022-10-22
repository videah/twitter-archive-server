mod guards;
mod templates;

#[macro_use] extern crate rocket;

use rocket::serde::json::Json;
use rocket::fs::FileServer;
use rocket_governor::{RocketGovernor, rocket_governor_catcher};

use twitter_v2::authorization::BearerToken;
use twitter_v2::data::MediaType;
use twitter_v2::query::{MediaField, TweetExpansion, TweetField};

use serde::Serialize;
use user_agent_parser::{OS, UserAgentParser};

/// Simplified response containing tweet data from the Twitter API.
/// Twitter's API is very verbose and kind of unwieldy, so we only return the fields we need.
#[derive(Serialize)]
struct NabResponse {
    /// Username of the tweet author.
    username: String,
    /// The tweet's text.
    text: String,
    /// List containing the filetypes of the media in the tweet in order.
    types: Vec<String>,
    /// List of photo/video URLs in their highest available quality included in the tweet, if any.
    media: Vec<String>,
}

use guards::RateLimitGuard;

#[get("/nab-tweet/<id>")]
async fn get_tweet_content(id: u64, _ratelimit: RocketGovernor<'_, RateLimitGuard>) -> Json<NabResponse> {
    let token = std::env::var("TWITTER_BEARER_TOKEN").expect("TWITTER_BEARER_TOKEN not set");
    let auth = BearerToken::new(token);
    let api = twitter_v2::TwitterApi::new(auth);

    let result = api.get_tweet(id)
        .tweet_fields([TweetField::AuthorId, TweetField::Text])
        .media_fields([MediaField::Type, MediaField::Url, MediaField::Variants])
        .expansions([TweetExpansion::AttachmentsMediaKeys])
        .send().await.expect("Failed to get tweet");

    let tweet = result.clone().into_data().unwrap();
    let includes = result.clone().into_includes().unwrap();
    let media = includes.media.unwrap();

    let users = api.get_users([tweet.author_id.unwrap()])
        .send().await
        .expect("Failed to get user")
        .into_data()
        .unwrap();

    let user = users.first().unwrap();

    let types = media.iter().map(|m|
        match m.kind {
            MediaType::Photo => "jpg".to_string(),
            MediaType::Video => "mp4".to_string(),
            MediaType::AnimatedGif => "mp4".to_string(),
        }
    ).collect();

    // Match media by type and put them in a matching Vec
    let mut media_urls = Vec::new();
    for m in media {
        match m.kind {
            MediaType::Photo => media_urls.push(format!("{}:orig", m.url.as_ref().unwrap())),
            MediaType::Video | MediaType::AnimatedGif => {
                let url = m.variants.unwrap().iter()
                    .max_by_key(|v| v.bit_rate.unwrap_or(0))
                    .unwrap().url.as_ref().unwrap().to_string();

                media_urls.push(url);
            },
        }
    }

    Json(
        NabResponse {
            username: user.username.clone(),
            text: tweet.text,
            types,
            media: media_urls,
        }
    )
}

#[get("/")]
async fn index(os: OS<'_>) -> templates::IndexTemplate {
    let os = os.name.unwrap_or_else(|| std::borrow::Cow::from("Unknown")).to_string();
    let show_install = matches!(os.as_str(), "Mac OS X" | "iOS");

    templates::IndexTemplate { show_install }
}

#[launch]
async fn rocket() -> _ {
    println!("Starting up twitter-archive-server... 🐔");

    rocket::build()
        .manage(UserAgentParser::from_str(include_str!("../regexes.yaml")).unwrap())
        .mount("/", routes![index])
        .mount("/api", routes![get_tweet_content])
        .mount("/static", FileServer::from("static"))
        .register("/", catchers!(rocket_governor_catcher))
}