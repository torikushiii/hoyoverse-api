use crate::types::{NewsItem, HoyolabResponse, EventItem, NewsListItem};
use reqwest::Client;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use tracing::error;

const SUPPORTED_LANGUAGES: [&str; 15] = [
    "en-us", "zh-cn", "zh-tw", "de-de", "es-es", "fr-fr", "id-id",
    "it-it", "ja-jp", "ko-kr", "pt-pt", "ru-ru", "th-th", "tr-tr", "vi-vn"
];

async fn fetch_api<T: DeserializeOwned>(client: &Client, url: &str, params: &HashMap<String, String>, lang: &str) -> anyhow::Result<Vec<T>> {
    let response = client.get(url)
        .query(params)
        .header("x-rpc-app_version", "2.42.0")
        .header("x-rpc-client_type", "4")
        .header("x-rpc-language", lang)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await?;

    let body = response.text().await?;
    
    match serde_json::from_str::<HoyolabResponse<T>>(&body) {
        Ok(parsed) => Ok(parsed.data.list),
        Err(e) => {
            error!("Failed to parse response for URL {}", url);
            error!("Parse error: {}", e);
            Err(e.into())
        }
    }
}

pub async fn fetch_news(category: &str) -> anyhow::Result<Vec<NewsItem>> {
    let client = Client::new();
    let mut all_news = Vec::new();

    for lang in SUPPORTED_LANGUAGES {
        match category {
            "events" => {
                let mut params = HashMap::new();
                params.insert("page_size".to_string(), "15".to_string());
                params.insert("size".to_string(), "15".to_string());
                params.insert("gids".to_string(), "6".to_string());

                let events: Vec<EventItem> = fetch_api(
                    &client,
                    "https://bbs-api-os.hoyolab.com/community/community_contribution/wapi/event/list",
                    &params,
                    lang
                ).await?;

                for event in events {
                    let id = event.id.clone();
                    all_news.push(NewsItem {
                        id: event.id,
                        title: event.name,
                        description: Some(event.desc),
                        created_at: event.create_at,
                        start_at: Some(event.start),
                        end_at: Some(event.end),
                        banner: vec![event.banner_url],
                        url: format!("https://www.hoyolab.com/article/{}", id),
                        r#type: "event".to_string(),
                        lang: lang.to_string(),
                    });
                }
            },
            "notices" | "info" => {
                let mut params = HashMap::new();
                params.insert("gids".to_string(), "6".to_string());
                params.insert("page_size".to_string(), "15".to_string());
                params.insert("type".to_string(), if category == "notices" { "1" } else { "3" }.to_string());

                let news: Vec<NewsListItem> = fetch_api(
                    &client,
                    "https://bbs-api-os.hoyolab.com/community/post/wapi/getNewsList",
                    &params,
                    lang
                ).await?;

                for item in news {
                    let post_id = item.post.post_id.clone();
                    let mut banner_urls = Vec::new();
                    
                    banner_urls.extend(
                        item.post.image_list
                            .into_iter()
                            .map(|img| img.url)
                    );
                    
                    banner_urls.extend(
                        item.image_list
                            .into_iter()
                            .map(|img| img.url)
                    );
                    
                    banner_urls.dedup();

                    all_news.push(NewsItem {
                        id: item.post.post_id,
                        title: item.post.subject,
                        description: Some(item.post.content),
                        created_at: item.post.created_at,
                        start_at: None,
                        end_at: None,
                        banner: banner_urls,
                        url: format!("https://www.hoyolab.com/article/{}", post_id),
                        r#type: category.to_string(),
                        lang: lang.to_string(),
                    });
                }
            },
            _ => tracing::warn!("Unsupported news category: {}", category),
        }
    }

    Ok(all_news)
} 