<p align="center">
  <img src="https://cdn.prod.website-files.com/68e09cef90d613c94c3671c0/697e805a9246c7e090054706_logo_horizontal_grey.png" alt="Yeti" width="200" />
</p>

---

# app-grafana

[![Yeti](https://img.shields.io/badge/Yeti-Application-blue)](https://yetirocks.com)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

> **[Yeti](https://yetirocks.com)** - The Performance Platform for Agent-Driven Development.
> Schema-driven APIs, real-time streaming, and vector search. From prompt to production.

Grafana SimpleJSON datasource for yeti -- query any yeti table directly from Grafana dashboards.

## Features

- **Grafana SimpleJSON protocol** -- works with the standard SimpleJSON datasource plugin
- **Query any yeti table** using `app_id/TableName` target format
- **Time series and table formats** -- auto-detects numeric fields for time series, derives columns for table view
- **Auto-column detection** -- table columns and types inferred from record structure
- **Auto-timestamp detection** -- checks `timestamp`, `createdAt`, `time`, `created_at`, `updatedAt` fields
- **Configurable base URL** and app filtering via DatasourceConfig table

## Installation

```bash
git clone https://github.com/yetirocks/app-grafana.git
cp -r app-grafana ~/yeti/applications/
```

## Project Structure

```
app-grafana/
  config.yaml
  schemas/
    schema.graphql
  resources/
    search.rs       # Connection test + target listing
    query.rs        # Data fetching in SimpleJSON format
```

## Configuration

```yaml
name: "Grafana Datasource"
app_id: "app-grafana"
version: "0.1.0"
description: "Grafana SimpleJSON datasource -- query yeti tables from Grafana dashboards"

schemas:
  - schemas/schema.graphql

resources:
  - resources/*.rs

auth:
  methods: [jwt, basic]
```

## Schema

**DatasourceConfig** -- Runtime configuration for the datasource.

```graphql
type DatasourceConfig @table(database: "app-grafana") @export {
    id: ID! @primaryKey              # "default"
    baseUrl: String                  # yeti base URL (default http://127.0.0.1:9996)
    allowedApps: String              # JSON array of app_ids to expose (empty = all)
    timeField: String                # default time field name (default "createdAt")
}
```

## API Reference

### GET /app-grafana/search

Connection test. Grafana calls this when you click "Save & Test" on the datasource.

```bash
curl https://localhost:9996/app-grafana/search \
  -H "Authorization: Bearer $TOKEN"

# Response
# 200 { "status": "ok", "message": "Yeti Grafana Datasource" }
```

### POST /app-grafana/search

List available query targets. Returns app IDs from the yeti health endpoint, optionally filtered by the `target` field.

```bash
curl -X POST https://localhost:9996/app-grafana/search \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{ "target": "" }'

# Response
# 200 ["app-siem", "app-prometheus", "yeti-telemetry", ...]
```

### POST /app-grafana/query

Fetch data in Grafana SimpleJSON format. Targets use the format `app_id/TableName`.

```bash
curl -X POST https://localhost:9996/app-grafana/query \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "targets": [
      { "target": "yeti-telemetry/Log", "type": "table" }
    ],
    "range": {
      "from": "2024-01-01T00:00:00Z",
      "to": "2024-12-31T00:00:00Z"
    },
    "maxDataPoints": 100
  }'
```

**Table response:**

```json
[{
  "type": "table",
  "columns": [
    { "text": "id", "type": "string" },
    { "text": "level", "type": "string" },
    { "text": "timestamp", "type": "number" }
  ],
  "rows": [
    ["log-001", "INFO", 1711700000],
    ["log-002", "ERROR", 1711700060]
  ]
}]
```

**Time series response** (use `"type": "timeserie"`):

```json
[{
  "target": "app-siem/CostTracking",
  "datapoints": [
    [0.045, 1711700000000],
    [0.120, 1711786400000]
  ]
}]
```

### Target Format

Targets follow the pattern `app_id/TableName`:

| Target | Description |
|--------|-------------|
| `yeti-telemetry/Log` | Telemetry log records |
| `yeti-telemetry/Span` | Telemetry span records |
| `app-siem/Event` | SIEM security events |
| `app-siem/CostTracking` | SIEM daily cost data |
| `app-rate-limiter/RequestLog` | Rate limiter request logs |

## Grafana Setup

1. Install the **SimpleJSON** datasource plugin in Grafana (or use the built-in JSON API datasource).

2. Add a new datasource:
   - **Type:** SimpleJSON
   - **URL:** `https://localhost:9996/app-grafana`
   - **Auth:** Check "With Credentials" or add a custom header:
     - Header: `Authorization`
     - Value: `Bearer <your-jwt-token>`
   - **TLS:** Enable "Skip TLS Verify" for self-signed dev certs

3. Click **Save & Test** -- you should see "Data source is working".

4. Create a new panel:
   - Select your Yeti datasource
   - Choose a target (e.g., `yeti-telemetry/Log`)
   - Set format to "Table" or "Time series"

---

Built with [Yeti](https://yetirocks.com) | The Performance Platform for Agent-Driven Development
