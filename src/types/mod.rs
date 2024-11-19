pub mod codes;
pub mod news;
pub mod calendar;
pub mod hoyolab;
pub mod starrail;
pub mod genshin;

pub use codes::{GameCode, GameCodeResponse, CodesResponse};
pub use news::{NewsItem, NewsItemResponse, NewsPost, NewsList, Post, ImageItem};
pub use calendar::CalendarResponse;
pub use hoyolab::{HoyolabResponse, HoyolabDataResponse, EventList, EventItem};
pub use starrail::{StarRailCalendarResponse, StarRailCalendarData};
pub use genshin::{GenshinCalendarResponse, GenshinCalendarData};