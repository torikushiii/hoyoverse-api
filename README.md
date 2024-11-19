# HoYoverse API

A high-performance REST API service providing redemption codes and news for HoYoverse games:
- Genshin Impact
- Honkai: Star Rail
- Honkai Impact 3rd
- Tears of Themis
- Zenless Zone Zero

## Base URL
```
https://api.ennead.cc/mihoyo
```

## Endpoints

All games follow the same endpoint pattern:

### Redemption Codes
```
GET /{game}/codes
```
Returns active and expired redemption codes.

### News
```
GET /{game}/news/{category}?lang={language}
```
Returns news articles for the specified category and language.

Where:
- `game`: `genshin`, `starrail`, `honkai`, `themis`, or `zenless`
- `category`: `notices`, `events`, or `info`
- `language`: `en` (default), `zh`, `ja`, etc.

## Response Examples

### Codes Response
```json
{
    "active": [
        {
            "code": "GENSHINGIFT",
            "reward": [
                "60 Primogems",
                "10000 Mora"
            ]
        }
    ],
    "inactive": [
        {
            "code": "OLDCODE123",
            "reward": [
                "30 Primogems"
            ]
        }
    ]
}
```

### News Response
```json
[
    {
        "id": "123",
        "title": "Version 4.5 Update Notice",
        "type": "notices",
        "url": "https://example.com/article/123",
        "banner": [
            "https://example.com/images/banner.jpg"
        ],
        "createdAt": 1731124812
    }
]
```

## Rate Limits
- 60 requests per minute per IP
- Exceeding this limit returns a 429 status code

## Error Codes

| Status Code | Error Code | Description |
|------------|------------|-------------|
| 200 | - | Success |
| 400 | 4000 | Bad Request |
| 404 | 3000 | Not Found |
| 429 | 2000 | Too Many Requests |
| 500 | 0 | Internal Server Error |

Error Response Format:
```json
{
    "status": "Not Found",
    "error_code": 3000,
    "error": "Resource not found"
}
```
