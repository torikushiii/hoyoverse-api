/// Parses a language code into its normalized form.
/// 
/// # Arguments
/// * `lang_code` - A language code string (e.g., "en", "jp", "kr")
/// 
/// # Returns
/// A normalized language code string (e.g., "en-us", "ja-jp", "ko-kr")
/// 
/// # Examples
/// ```
/// let result = hoyoverse_api::utils::lang::parse_language_code("en");
/// assert_eq!(result, "en-us");
/// 
/// let result = hoyoverse_api::utils::lang::parse_language_code("jp");
/// assert_eq!(result, "ja-jp");
/// 
/// let result = hoyoverse_api::utils::lang::parse_language_code("kr");
/// assert_eq!(result, "ko-kr");
/// ```
pub fn parse_language_code(lang_code: &str) -> &'static str {
    match lang_code.trim().to_lowercase().as_str() {
        "en" => "en-us",
        "cn" => "zh-cn",
        "tw" => "zh-tw",
        "de" => "de-de",
        "es" => "es-es",
        "fr" => "fr-fr",
        "id" => "id-id",
        "it" => "it-it",
        "ja" | "jp" => "ja-jp",
        "ko" | "kr" => "ko-kr",
        "pt" => "pt-pt",
        "ru" => "ru-ru",
        "th" => "th-th",
        "tr" => "tr-tr",
        "vi" | "vn" => "vi-vn",
        _ => "en-us",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_language_code() {
        // Test basic language codes
        assert_eq!(parse_language_code("en"), "en-us");
        assert_eq!(parse_language_code("cn"), "zh-cn");
        assert_eq!(parse_language_code("tw"), "zh-tw");

        // Test alternative codes
        assert_eq!(parse_language_code("jp"), "ja-jp");
        assert_eq!(parse_language_code("ja"), "ja-jp");
        assert_eq!(parse_language_code("kr"), "ko-kr");
        assert_eq!(parse_language_code("ko"), "ko-kr");
        assert_eq!(parse_language_code("vn"), "vi-vn");
        assert_eq!(parse_language_code("vi"), "vi-vn");

        // Test case insensitivity
        assert_eq!(parse_language_code("EN"), "en-us");
        assert_eq!(parse_language_code("Jp"), "ja-jp");
        assert_eq!(parse_language_code("Kr"), "ko-kr");

        // Test whitespace handling
        assert_eq!(parse_language_code(" en "), "en-us");
        assert_eq!(parse_language_code("jp "), "ja-jp");
        assert_eq!(parse_language_code(" kr"), "ko-kr");

        // Test default fallback
        assert_eq!(parse_language_code("xx"), "en-us");
        assert_eq!(parse_language_code(""), "en-us");
    }
} 