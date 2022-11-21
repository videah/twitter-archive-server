use askama::Template;

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    pub show_install: bool,
}

#[derive(Template)]
#[template(path = "index-mastodon.html")]
pub struct MastodonTemplate {
    pub show_install: bool,
}