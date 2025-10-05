use crate::{config::Settings, types::*};
use reqwest::Client;
use serde::de::DeserializeOwned;
use tracing::{debug, error};

pub const SUPPORTED_LANGUAGES: [&str; 15] = [
    "en-us", "zh-cn", "zh-tw", "de-de", "es-es", "fr-fr", "id-id", "it-it", "ja-jp", "ko-kr",
    "pt-pt", "ru-ru", "th-th", "tr-tr", "vi-vn",
];

pub struct NewsResolver {
    client: Client,
    user_agent: String,
}

impl NewsResolver {
    pub fn new(config: &Settings) -> Self {
        Self {
            client: Client::new(),
            user_agent: config.server.user_agent.clone(),
        }
    }

    async fn fetch_data<T: DeserializeOwned>(
        &self,
        url: &str,
        params: &[(&str, &str)],
        lang: &str,
    ) -> anyhow::Result<T> {
        let response = self
            .client
            .get(url)
            .query(params)
            .header("x-rpc-app_version", "2.42.0")
            .header("x-rpc-client_type", "4")
            .header("x-rpc-language", lang)
            .header("User-Agent", &self.user_agent)
            .send()
            .await?;

        let data = response.json::<HoyolabDataResponse<T>>().await?;
        Ok(data.data)
    }

    pub async fn fetch_news(&self, lang: &str) -> anyhow::Result<Vec<NewsItem>> {
        let mut all_news = Vec::new();

        match self
            .fetch_data::<EventList>(
                "https://bbs-api-os.hoyolab.com/community/community_contribution/wapi/event/list",
                &[("page_size", "15"), ("size", "15"), ("gids", "4")],
                lang,
            )
            .await
        {
            Ok(events) => {
                debug!(
                    "[Themis] Fetched {} events for language {}",
                    events.list.len(),
                    lang
                );
                for item in events.list {
                    if item.id.is_empty() || item.name.is_empty() {
                        error!(
                            "[Themis] Skipping event with empty id or name for lang {}",
                            lang
                        );
                        continue;
                    }

                    let id = item.id.clone();
                    all_news.push(NewsItem {
                        id: None,
                        external_id: item.id,
                        title: item.name,
                        description: item.desc,
                        created_at: item.create_at,
                        banner: Some(vec![item.banner_url]),
                        url: format!("https://www.hoyolab.com/article/{}", id),
                        news_type: "event".to_string(),
                        lang: lang.to_string(),
                    });
                }
            }
            Err(e) => {
                error!(
                    "[Themis] Failed to fetch events for language {}: {}",
                    lang, e
                );
            }
        }

        for (type_id, type_name) in [(1, "notice"), (3, "info")] {
            match self
                .fetch_data::<NewsList>(
                    "https://bbs-api-os.hoyolab.com/community/post/wapi/getNewsList",
                    &[
                        ("gids", "4"),
                        ("page_size", "15"),
                        ("type", &type_id.to_string()),
                    ],
                    lang,
                )
                .await
            {
                Ok(news) => {
                    debug!(
                        "[Themis] Fetched {} {} for language {}",
                        news.list.len(),
                        type_name,
                        lang
                    );
                    for item in news.list {
                        if item.post.post_id.is_empty() || item.post.subject.is_empty() {
                            error!(
                                "[Themis] Skipping {} with empty id or subject for lang {}",
                                type_name, lang
                            );
                            continue;
                        }

                        let post_id = item.post.post_id.clone();
                        all_news.push(NewsItem {
                            id: None,
                            external_id: item.post.post_id,
                            title: item.post.subject,
                            description: item.post.content,
                            created_at: item.post.created_at,
                            banner: if item.image_list.is_empty() {
                                None
                            } else {
                                Some(item.image_list.into_iter().map(|img| img.url).collect())
                            },
                            url: format!("https://www.hoyolab.com/article/{}", post_id),
                            news_type: type_name.to_string(),
                            lang: lang.to_string(),
                        });
                    }
                }
                Err(e) => {
                    error!(
                        "[Themis] Failed to fetch {} for language {}: {}",
                        type_name, lang, e
                    );
                }
            }
        }

        debug!(
            "[Themis] Total news items fetched for {}: {}",
            lang,
            all_news.len()
        );
        Ok(all_news)
    }
}

pub async fn fetch_news(config: &Settings, _category: &str) -> anyhow::Result<Vec<NewsItem>> {
    let resolver = NewsResolver::new(config);
    let mut all_news = Vec::new();

    for lang in SUPPORTED_LANGUAGES {
        match resolver.fetch_news(lang).await {
            Ok(mut news) => all_news.append(&mut news),
            Err(e) => error!("[Themis] Failed to fetch news for language {}: {}", lang, e),
        }
    }

    Ok(all_news)
}
