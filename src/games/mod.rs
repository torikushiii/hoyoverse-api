pub mod genshin;
pub mod starrail;
pub mod themis;
pub mod zenless;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Game {
    Genshin,
    Starrail,
    Zenless,
    Honkai,
    Themis,
}

impl Game {
    pub fn collection_name(&self) -> &'static str {
        match self {
            Self::Genshin => "genshin_codes",
            Self::Starrail => "starrail_codes",
            Self::Zenless => "zenless_codes",
            Self::Honkai => "honkai_codes",
            Self::Themis => "themis_codes",
        }
    }

    pub fn slug(&self) -> &'static str {
        match self {
            Self::Genshin => "genshin",
            Self::Starrail => "starrail",
            Self::Zenless => "zenless",
            Self::Honkai => "honkai",
            Self::Themis => "themis",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        match s {
            "genshin" => Some(Self::Genshin),
            "starrail" => Some(Self::Starrail),
            "zenless" => Some(Self::Zenless),
            "honkai" => Some(Self::Honkai),
            "themis" => Some(Self::Themis),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Genshin => "Genshin Impact",
            Self::Starrail => "Honkai: Star Rail",
            Self::Zenless => "Zenless Zone Zero",
            Self::Honkai => "Honkai Impact 3rd",
            Self::Themis => "Tears of Themis",
        }
    }

    pub fn redeem_endpoint(&self) -> Option<&'static str> {
        match self {
            Self::Genshin => Some(genshin::REDEEM_API),
            Self::Starrail => Some(starrail::REDEEM_API),
            Self::Zenless => Some(zenless::REDEEM_API),
            Self::Themis => Some(themis::REDEEM_API),
            _ => None,
        }
    }

    pub fn hoyolab_gid(&self) -> u32 {
        match self {
            Self::Honkai => 1,
            Self::Genshin => 2,
            Self::Themis => 4,
            Self::Starrail => 6,
            Self::Zenless => 8,
        }
    }

    pub fn embed_color(&self) -> u32 {
        match self {
            Self::Genshin => 0xF4D442,
            Self::Starrail => 0x9C59D1,
            Self::Zenless => 0xF4A614,
            Self::Honkai => 0xD72B23,
            Self::Themis => 0xC44FA4,
        }
    }

    pub fn game_biz(&self) -> Option<&'static str> {
        match self {
            Self::Genshin => Some(genshin::GAME_BIZ),
            Self::Starrail => Some(starrail::GAME_BIZ),
            Self::Zenless => Some(zenless::GAME_BIZ),
            Self::Themis => Some(themis::GAME_BIZ),
            _ => None,
        }
    }
}
