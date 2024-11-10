use chrono::{DateTime, Utc};
use mongodb::bson::DateTime as BsonDateTime;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub trait DateTimeExt {
    fn to_bson_datetime(&self) -> BsonDateTime;
}

impl DateTimeExt for DateTime<Utc> {
    fn to_bson_datetime(&self) -> BsonDateTime {
        self.clone().into()
    }
}

static START_TIME: AtomicI64 = AtomicI64::new(0);

pub fn set_start_time() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    START_TIME.store(now, Ordering::SeqCst);
}

pub fn get_uptime() -> i64 {
    START_TIME.load(Ordering::SeqCst)
}