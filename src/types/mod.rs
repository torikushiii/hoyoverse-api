pub mod calendar;
pub mod codes;
pub mod genshin;
pub mod hoyolab;
pub mod news;
pub mod starrail;

pub use calendar::CalendarResponse;
pub use codes::{CodesResponse, GameCode, GameCodeResponse};
pub use genshin::{GenshinCalendarData, GenshinCalendarResponse};
pub use hoyolab::{EventItem, EventList, HoyolabDataResponse, HoyolabResponse};
pub use news::{ImageItem, NewsItem, NewsItemResponse, NewsList, NewsPost, Post};
pub use starrail::{StarRailCalendarData, StarRailCalendarResponse};
