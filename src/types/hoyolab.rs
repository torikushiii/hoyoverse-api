use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct HoyolabResponse {
    pub retcode: i32,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct HoyolabDataResponse<T> {
    pub retcode: i32,
    pub message: String,
    pub data: T,
}

#[derive(Debug, Deserialize)]
pub struct EventList {
    pub list: Vec<EventItem>,
}

#[derive(Debug, Deserialize)]
pub struct EventItem {
    pub id: String,
    pub name: String,
    pub desc: String,
    #[serde(deserialize_with = "deserialize_timestamp")]
    pub create_at: i64,
    pub banner_url: String,
}

pub fn deserialize_timestamp<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum TimestampFormat {
        String(String),
        Integer(i64),
    }

    match TimestampFormat::deserialize(deserializer)? {
        TimestampFormat::String(s) => s
            .parse::<i64>()
            .map_err(|e| Error::custom(format!("Failed to parse string timestamp: {}", e))),
        TimestampFormat::Integer(i) => Ok(i),
    }
}
