# HoYoverse API

A high-performance REST API service that provides redemption codes and news for HoYoverse games including Genshin Impact, Honkai: Star Rail, and Zenless Zone Zero.

## API Endpoints

### Base URL
```
https://api.ennead.cc/mihoyo
```

### Available Endpoints

#### Genshin Impact
- **Get Redemption Codes**
  ```
  GET /genshin/codes
  ```
  Returns active and inactive redemption codes for Genshin Impact.

- **Get News**
  ```
  GET /genshin/news/{category}?lang={language}
  ```
  Categories: `notices`, `events`, `info`
  Languages: `en` (default), `zh`, `ja`, etc.

#### Honkai: Star Rail
- **Get Redemption Codes**
  ```
  GET /starrail/codes
  ```
  Returns active and inactive redemption codes for Star Rail.

- **Get News**
  ```
  GET /starrail/news/{category}?lang={language}
  ```
  Categories: `notices`, `events`, `info`
  Languages: `en` (default), `zh`, `ja`, etc.

#### Zenless Zone Zero
- **Get Redemption Codes**
  ```
  GET /zenless/codes
  ```
  Returns active and inactive redemption codes for Zenless Zone Zero.

- **Get News**
  ```
  GET /zenless/news/{category}?lang={language}
  ```
  Categories: `notices`, `events`, `info`
  Languages: `en` (default), `zh`, `ja`, etc.

## Response Format

### Codes Endpoint Response

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

### News Endpoint Response

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
        "createdAt": 1731124812,
    }
]
```

## Rate Limiting

The API implements rate limiting to ensure fair usage:
- 60 requests per minute per IP address
- Exceeding this limit will result in a `429 Too Many Requests` response

## Error Responses

The API uses standard HTTP status codes:
- `200`: Success
- `400`: Bad Request
- `429`: Too Many Requests
- `500`: Internal Server Error