use crate::types::*;
use reqwest::Client;
use tracing::{error, debug};

pub const SUPPORTED_LANGUAGES: [&str; 15] = [
    "en-us", "zh-cn", "zh-tw", "de-de", "es-es", "fr-fr", "id-id",
    "it-it", "ja-jp", "ko-kr", "pt-pt", "ru-ru", "th-th", "tr-tr", "vi-vn"
];

pub struct NewsResolver {
    client: Client,
}

impl NewsResolver {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    async fn fetch_api<T>(&self, url: &str, params: &[(&str, &str)], lang: &str) -> anyhow::Result<T> 
    where
        T: serde::de::DeserializeOwned,
    {
        let response = self.client
            .get(url)
            .query(params)
            .header("x-rpc-app_version", "2.42.0")
            .header("x-rpc-client_type", "4")
            .header("x-rpc-language", lang)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36")
            .send()
            .await?;

        let data = response.json::<HoyolabResponse<T>>().await?;
        Ok(data.data)
    }

    pub async fn fetch_news(&self, lang: &str) -> anyhow::Result<Vec<NewsItem>> {
        let mut all_news = Vec::new();
        
        // Fetch events
        match self.fetch_api::<EventList>(
            "https://bbs-api-os.hoyolab.com/community/community_contribution/wapi/event/list",
            &[
                ("page_size", "15"),
                ("size", "15"),
                ("gids", "6"),
            ],
            lang
        ).await {
            Ok(events) => {
                debug!("[StarRail] Fetched {} events for language {}", events.list.len(), lang);
                for item in events.list {
                    if item.id.is_empty() || item.name.is_empty() {
                        error!("[StarRail] Skipping event with empty id or name for lang {}", lang);
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
                error!("[StarRail] Failed to fetch events for language {}: {}", lang, e);
            }
        }

        // Fetch notices and info
        for (type_id, type_name) in [(1, "notice"), (3, "info")] {
            match self.fetch_api::<NewsList>(
                "https://bbs-api-os.hoyolab.com/community/post/wapi/getNewsList",
                &[
                    ("gids", "6"),
                    ("page_size", "15"),
                    ("type", &type_id.to_string()),
                ],
                lang
            ).await {
                Ok(news) => {
                    debug!("[StarRail] Fetched {} {} for language {}", news.list.len(), type_name, lang);
                    for item in news.list {
                        if item.post.post_id.is_empty() || item.post.subject.is_empty() {
                            error!("[StarRail] Skipping {} with empty id or subject for lang {}", type_name, lang);
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
                    error!("[StarRail] Failed to fetch {} for language {}: {}", type_name, lang, e);
                }
            }
        }

        debug!("[StarRail] Total news items fetched for {}: {}", lang, all_news.len());
        Ok(all_news)
    }
}

pub async fn fetch_news(_category: &str) -> anyhow::Result<Vec<NewsItem>> {
    let resolver = NewsResolver::new();
    let mut all_news = Vec::new();

    for lang in SUPPORTED_LANGUAGES {
        match resolver.fetch_news(lang).await {
            Ok(mut news) => all_news.append(&mut news),
            Err(e) => error!("[StarRail] Failed to fetch news for language {}: {}", lang, e),
        }
    }

    Ok(all_news)
} 