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

### Calendar (Genshin Impact and Star Rail only)
```
GET /{game}/calendar
```
Returns current events, banners and challenges for the game. Only available for `genshin` and `starrail`.

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

### Calendar Response
```json
{
    "events": [
        {
            "id": 46,
            "name": "Gift of Odyssey",
            "description": "",
            "type_name": "ActivityTypeSign",
            "start_time": 1729479600,
            "end_time": 1733194799,
            "rewards": [
                {
                    "id": 102,
                    "name": "Star Rail Special Pass",
                    "icon": "https://example.com/icon.png",
                    "rarity": "5",
                    "amount": 10
                }
            ]
        }
    ],
    "banners": [
        {
            "id": "12",
            "name": "",
            "version": "2.6",
            "characters": [
                {
                    "id": "1308",
                    "name": "Acheron",
                    "rarity": "5",
                    "element": "lightning"
                }
            ],
            "start_time": 1731495600,
            "end_time": 1733234400
        }
    ],
    "challenges": [
        {
            "id": 2010,
            "name": "Rumor Mill",
            "type_name": "ChallengeTypeStory",
            "start_time": 1731294000,
            "end_time": 1734922800
        }
    ]
}
```

## Rate Limits
- 60 requests per minute per IP
- Exceeding this limit returns a 429 status code

## Reporting Invalid Codes
If you find any redemption codes that are incorrectly parsed or have wrong reward information, please [create an issue](https://github.com/torikushiii/hoyoverse-api/issues/new) with:
- The specific code
- The game it's for (Genshin, Star Rail, etc.)
- What's incorrect (wrong rewards, parsing error, etc.)

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
