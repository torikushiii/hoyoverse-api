pub mod crimson_witch;
pub mod genshin;
pub mod honkai;
pub mod starrail;
pub mod themis;
pub mod zenless;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCode {
    pub code: String,
    pub rewards: Vec<String>,
}
