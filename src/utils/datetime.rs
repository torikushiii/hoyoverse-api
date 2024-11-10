use chrono::{DateTime, Utc};
use mongodb::bson::DateTime as BsonDateTime;

pub trait DateTimeExt {
    fn to_bson_datetime(&self) -> BsonDateTime;
}

impl DateTimeExt for DateTime<Utc> {
    fn to_bson_datetime(&self) -> BsonDateTime {
        self.clone().into()
    }
}