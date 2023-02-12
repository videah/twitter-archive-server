mod guards;
mod templates;
mod models;

#[macro_use] extern crate rocket;

use std::future::Future;
use std::pin::Pin;
use futures_util::StreamExt;
use rocket::Either;
use rocket::Either::{Left, Right};

use rocket::serde::json::Json;
use rocket::fs::FileServer;
use rocket::http::uri::Host;
use rocket_governor::{RocketGovernor, rocket_governor_catcher};

use serde::Serialize;
use serde_json::Value;
use tweet_scraper::{HeaderPersist, TweetScraper};
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
use crate::templates::{IndexTemplate, MastodonTemplate};

fn scrape_tweet(query: &str, try_again: bool) -> Pin<Box<dyn Future<Output = Option<Value>> + Send + '_>> {
    Box::pin(async move {
        // Check if header file already exists in /tmp/
        let header_file = "/tmp/twitter_headers";
        let header_file_exists = std::path::Path::new(header_file).exists();

        let header_persist = if header_file_exists {
            // If it does, load the headers from the file.
            HeaderPersist::Load(header_file.parse().unwrap())
        } else {
            // If it doesn't, create a new header file.
            println!("Creating new header file");
            HeaderPersist::Save(header_file.parse().unwrap())
        };

        let mut scraper = match TweetScraper::initialize(header_persist).await {
            Ok(scraper) => scraper,
            Err(e) => panic!("Error initializing TweetScraper: {}", e),
        };

        let tweets_stream = scraper.tweets(query, Some(1), None).await;
        futures_util::pin_mut!(tweets_stream);

        let tweet = tweets_stream.next().await;

        let scraped_tweet = match tweet {
            Some(Ok(tweet)) => {
                Some(tweet)
            },
            Some(Err(e)) => {
                if try_again {
                    println!("Error scraping tweet: {}", e);
                    println!("It's possible that the guest token expired, so we'll try again.");
                    // Delete the header file so we can get a new guest token.
                    std::fs::remove_file(header_file).unwrap();
                    println!("Trying again...");
                    scrape_tweet(query, false).await
                } else {
                    println!("Error scraping tweet: {}", e);
                    println!("Giving up...");
                    None
                }
            },
            None => None,
        };

        scraped_tweet
    })
}

#[get("/nab-tweet/<id>")]
async fn get_tweet_content(id: u128, _ratelimit: RocketGovernor<'_, RateLimitGuard>) -> Json<NabResponse> {

    let before = id - 1;
    let query_safe = format!("since_id:{before} max_id:{id} filter:safe");
    let query_unsafe = format!("since_id:{before} max_id:{id} -filter:safe");
    println!("{query_safe}");

    // Tweet query's cant just be since_id and max_id so we use filter:safe to make the query valid.
    // This means we have to potentially make another query if the tweet is not "safe".
    let tweet = match scrape_tweet(&query_safe, true).await {
        Some(tweet) => tweet,
        None => scrape_tweet(&query_unsafe, true).await.unwrap(),
    };

    let scraped_tweet: models::ScrapedTweet = serde_json::from_value(tweet).unwrap();

    let mut type_list = vec![];
    let mut media_list = vec![];
    if let Some(entities) = scraped_tweet.extended_entities {
        entities.media.iter().for_each(|media| {
            println!("Media: {}", media.media_type);
            match media.media_type.as_str() {
                "animated_gif" | "video" => {
                    type_list.push("mp4".to_string());
                    // Get the highest quality video URL from the variants list.
                    media_list.push(media.video_info.as_ref().unwrap().variants.iter().max_by_key(
                        |variant| variant.bitrate
                    ).unwrap().url.clone());
                },
                "photo" => {
                    type_list.push("jpg".to_string());
                    media_list.push(format!("{}:orig", media.media_url_https));
                },
                _ => type_list.push("unknown".to_string()),
            }
        });
    }

    Json(
        NabResponse {
            username: scraped_tweet.user.screen_name,
            text: scraped_tweet.full_text,
            types: type_list,
            media: media_list,
        }
    )
}

#[get("/")]
async fn index(os: OS<'_>, host: &Host<'_>) -> Either<IndexTemplate, MastodonTemplate> {
    let os = os.name.unwrap_or_else(|| std::borrow::Cow::from("Unknown")).to_string();
    let show_install = matches!(os.as_str(), "Mac OS X" | "iOS");

    if host.to_string().eq("mastodon-archive.club") {
        Right(MastodonTemplate { show_install })
    } else {
        Left(IndexTemplate { show_install })
    }
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