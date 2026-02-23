# HoYoverse API

A REST API that provides redemption codes, event calendars, and news for HoYoverse games.

## Supported Games

| Game | Slug |
|------|------|
| Genshin Impact | `genshin` |
| Honkai: Star Rail | `starrail` |
| Zenless Zone Zero | `zenless` |
| Honkai Impact 3rd | `honkai` |
| Tears of Themis | `themis` |

## Base URL

All endpoints are prefixed with `/mihoyo`.

## Endpoints

### API Info

```
GET /mihoyo/
```

Returns API version, uptime, and a list of available endpoints.

**Response:**

```json
{
  "message": "HoYoverse Redemption Code API",
  "version": "1.0.0",
  "uptime": 3600,
  "endpoints": [...]
}
```

---

### Redemption Codes

```
GET /mihoyo/{game}/codes
```

Returns active and inactive redemption codes for the specified game.

**Path Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `game` | string | Game slug (see supported games table) |

**Response:**

```json
{
  "active": [
    {
      "code": "GENSHINGIFT",
      "rewards": ["Primogems x60", "Mora x10000"]
    }
  ],
  "inactive": [
    {
      "code": "OLDCODE123",
      "rewards": ["Primogems x30"]
    }
  ]
}
```

---

### Event Calendar

Available for **Genshin Impact** and **Honkai: Star Rail** only.

```
GET /mihoyo/genshin/calendar
GET /mihoyo/starrail/calendar
```

Returns current events, character/weapon banners, and challenges.

**Genshin Impact Response:**

```json
{
  "events": [
    {
      "id": 1,
      "name": "Event Name",
      "description": "Event description",
      "image_url": "https://...",
      "type_name": "In-Game",
      "start_time": 1700000000,
      "end_time": 1700100000,
      "rewards": [
        {
          "id": 1,
          "name": "Primogem",
          "icon": "https://...",
          "rarity": "5",
          "amount": 420
        }
      ],
      "special_reward": null
    }
  ],
  "banners": [
    {
      "id": 1,
      "name": "Banner Name",
      "version": "4.5",
      "characters": [
        {
          "id": 1,
          "name": "Character Name",
          "icon": "https://...",
          "element": "Pyro",
          "rarity": 5
        }
      ],
      "weapons": [
        {
          "id": 1,
          "name": "Weapon Name",
          "icon": "https://...",
          "rarity": 5
        }
      ],
      "start_time": 1700000000,
      "end_time": 1700100000
    }
  ],
  "challenges": [
    {
      "id": 1,
      "name": "Spiral Abyss",
      "type_name": "Abyss",
      "start_time": 1700000000,
      "end_time": 1700100000,
      "rewards": [],
      "special_reward": null
    }
  ]
}
```

The **Star Rail** calendar response follows the same structure but with `light_cones` instead of `weapons`, and characters/light cones include an additional `path` field.

---

### News

```
GET /mihoyo/{game}/news/events
GET /mihoyo/{game}/news/notices
GET /mihoyo/{game}/news/info
```

Returns the latest HoYoLab community posts for the specified game, categorized by type.

**Path Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `game` | string | Game slug (see supported games table) |

**Query Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `lang` | string | `en-us` | Language for news content |

**Supported languages:**

| Code | Full locale |
|------|-------------|
| `en` / `en-us` | English |
| `zh` / `zh-cn` | Chinese (Simplified) |
| `zh-tw` | Chinese (Traditional) |
| `de` / `de-de` | German |
| `es` / `es-es` | Spanish |
| `fr` / `fr-fr` | French |
| `id` / `id-id` | Indonesian |
| `it` / `it-it` | Italian |
| `ja` / `ja-jp` | Japanese |
| `ko` / `ko-kr` | Korean |
| `pt` / `pt-pt` | Portuguese |
| `ru` / `ru-ru` | Russian |
| `th` / `th-th` | Thai |
| `tr` / `tr-tr` | Turkish |
| `vi` / `vi-vn` | Vietnamese |

**Response:**

```json
[
  {
    "id": "12345",
    "title": "Version 4.5 Update Notice",
    "description": "Dear Travelers, below are the details of the Version 4.5 update...",
    "created_at": 1700000000,
    "banner": "https://...",
    "url": "https://www.hoyolab.com/article/12345",
    "type": "notice"
  }
]
```

The `type` field matches the endpoint used: `"event"`, `"notice"`, or `"info"`.

---

## Error Handling

All errors follow a consistent format:

```json
{
  "status": "Not Found",
  "error_code": 1000,
  "error": "Unknown game: invalid_game"
}
```

**Error Codes:**

| Code | Name | Description |
|------|------|-------------|
| 404 | ROUTE_NOT_FOUND | The requested endpoint does not exist |
| 1000 | UNKNOWN_GAME | The game slug is not recognized |
| 1001 | INVALID_LANGUAGE | The `lang` parameter is not a supported language |
| 2000 | DATABASE_ERROR | A database operation failed |
| 3000 | NOT_CONFIGURED | The requested feature is not configured on the server |
| 3001 | UPSTREAM_ERROR | An upstream HoYoverse/HoYoLab API call failed |

## Rate Limiting

The API enforces IP-based rate limiting. Default limits are **2 requests per second** with a burst allowance of **120 requests**. Requests exceeding the limit will receive a `429 Too Many Requests` response.

## Caching

Responses are cached in memory to reduce load on upstream services:

- **Redemption codes:** 5 minutes
- **Calendar data:** 5 minutes
- **News:** 15 minutes

## Reporting Invalid Codes

If you find any redemption codes that are incorrectly parsed or have wrong reward information, please [create an issue](../../issues/new) and include the following:

- The **redemption code** in question
- The **game** it belongs to
- A description of **what is incorrect** (e.g., wrong rewards listed, code marked as active when expired, etc.)
