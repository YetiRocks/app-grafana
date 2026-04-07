<p align="center">
  <img src="https://cdn.prod.website-files.com/68e09cef90d613c94c3671c0/697e805a9246c7e090054706_logo_horizontal_grey.png" alt="Yeti" width="200" />
</p>

---

# app-grafana

[![Yeti](https://img.shields.io/badge/Yeti-Application-blue)](https://yetirocks.com)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

> **[Yeti](https://yetirocks.com)** - The Performance Platform for Agent-Driven Development.
> Schema-driven APIs, real-time streaming, and vector search. From prompt to production.

**A Grafana datasource for every yeti table.** Zero plugins, zero toolchain, zero configuration.

Query any table from any yeti application directly in Grafana dashboards -- time series, tables, logs, metrics, events. No Go compiler, no TypeScript bundler, no Grafana plugin SDK. One yeti application implements the full SimpleJSON protocol and bridges Grafana to every table in your stack.

---

## Why app-grafana

Building a Grafana datasource plugin the traditional way requires a Go backend, a TypeScript frontend, the Grafana plugin SDK, `mage` builds, `yarn` builds, plugin signing, and a restart of Grafana for every change. That is four toolchains for one integration.

app-grafana collapses all of that into a single yeti application:

- **SimpleJSON protocol** -- implements the standard Grafana SimpleJSON datasource protocol. No custom plugin installation. Works with the SimpleJSON plugin that ships in every Grafana instance.
- **Query any table** -- targets use the `app_id/TableName` format. Add a new yeti application and its tables appear in Grafana automatically.
- **Table and time series** -- auto-detects column types for table format, finds numeric fields for time series. No schema mapping required.
- **Auto-column detection** -- table columns and types are inferred from record structure. Add a field to your schema, it appears in Grafana on the next query.
- **Auto-timestamp detection** -- scans `timestamp`, `createdAt`, `time`, `created_at`, `updatedAt` fields. Handles both seconds and milliseconds.
- **Configurable filtering** -- restrict which apps are visible via `allowedApps` in the DatasourceConfig table. Empty means all apps are exposed.
- **Single binary deployment** -- compiles into a native Rust plugin. No Node.js, no npm, no Docker compose. Loads with yeti in seconds.

---

## Quick Start

### 1. Install

```bash
cd ~/yeti/applications
git clone https://github.com/yetirocks/app-grafana.git
```

Restart yeti. app-grafana compiles automatically on first load (~2 minutes) and is cached for subsequent starts (~10 seconds).

### 2. Test the connection

```bash
curl -k https://localhost/app-grafana/api/search \
  -H "Authorization: Bearer $TOKEN"
```

Response:
```json
{"status": "ok", "message": "Yeti Grafana Datasource"}
```

If you see `{"status": "ok"}`, the datasource is running and ready for Grafana.

### 3. Add the datasource in Grafana

1. Open Grafana (default `http://localhost:3000`)
2. Navigate to **Configuration** (gear icon) > **Data Sources** > **Add data source**
3. Search for **SimpleJSON** and select it
4. Configure the connection:
   - **Name:** `Yeti`
   - **URL:** `https://localhost/app-grafana`
   - **Access:** `Server (default)`
5. Under **Auth**, check **Skip TLS Verify** (for self-signed dev certs)
6. Under **Custom HTTP Headers**, add:
   - **Header:** `Authorization`
   - **Value:** `Bearer <your-jwt-token>`
7. Click **Save & Test** -- you should see a green banner: "Data source is working"

### 4. Create your first panel

1. Create a new dashboard (**+** icon > **Dashboard** > **Add new panel**)
2. Select your **Yeti** datasource from the dropdown
3. In the **Metric** field, type a target (e.g., `yeti-telemetry/Log`)
4. Set the **Format** to `Table` for log-style data, or `Time series` for graphing numeric values
5. Click **Apply** -- your yeti data appears in the panel

---

## Architecture

```
Grafana Dashboard
    |
    +-- SimpleJSON protocol (HTTP)
    |
    v
+---------------------------------------------------+
|                   app-grafana                      |
|                                                    |
|  GET  /search -----> connection test               |
|  POST /search -----> fetch /health                 |
|                      -> parse applicationList      |
|                      -> return app_id list          |
|                                                    |
|  POST /query ------> parse targets array           |
|                      -> for each "app_id/Table":   |
|                         fetch(yeti REST API)       |
|                         -> transform to Grafana    |
|                            SimpleJSON format       |
|                         -> table or timeseries     |
|                                                    |
|  DatasourceConfig --> baseUrl, allowedApps,        |
|                       timeField                    |
+---------------------------------------------------+
    |
    v
Yeti REST API (http://127.0.0.1)
    |
    +-- GET /health           -> application list
    +-- GET /{app}/{Table}    -> table records
```

**Query path:** Grafana panel request -> app-grafana `/query` endpoint -> parse `app_id/TableName` targets -> `fetch()` yeti REST API for each target -> transform records to Grafana SimpleJSON format (table columns/rows or time series datapoints) -> return JSON response to Grafana.

**Discovery path:** Grafana metric dropdown -> app-grafana `/search` endpoint -> `fetch()` yeti `/health` -> extract `applicationList` -> return filtered app IDs for target selection.

---

## Features

### Connection Test (GET /search)

Grafana calls `GET /search` when you click **Save & Test** on the datasource configuration page. app-grafana returns a simple status response:

```bash
curl -k https://localhost/app-grafana/api/search \
  -H "Authorization: Bearer $TOKEN"
```

Response:
```json
{"status": "ok", "message": "Yeti Grafana Datasource"}
```

Grafana interprets any 200 response as a successful connection.

### Target Discovery (POST /search)

Grafana calls `POST /search` to populate the metric/target dropdown in the panel editor. app-grafana fetches the application list from the yeti `/health` endpoint and returns matching app IDs:

```bash
curl -k -X POST https://localhost/app-grafana/api/search \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{ "target": "" }'
```

Response:
```json
["app-siem", "app-cortex", "app-grafana", "yeti-auth", "yeti-telemetry"]
```

The `target` field acts as a filter -- only app IDs containing the filter string (case-insensitive) are returned. An empty string returns all apps.

### Data Query (POST /query)

Grafana calls `POST /query` to fetch actual data for panels. Each target in the request specifies an `app_id/TableName` combination and a format type.

#### Table Format

Request:
```json
{
  "targets": [
    { "target": "yeti-telemetry/Log", "type": "table" }
  ],
  "range": {
    "from": "2024-01-01T00:00:00Z",
    "to": "2024-12-31T00:00:00Z"
  },
  "maxDataPoints": 100
}
```

Response:
```json
[{
  "type": "table",
  "columns": [
    { "text": "id", "type": "string" },
    { "text": "level", "type": "string" },
    { "text": "message", "type": "string" },
    { "text": "timestamp", "type": "number" }
  ],
  "rows": [
    ["log-001", "INFO", "Server started", 1711700000],
    ["log-002", "ERROR", "Connection refused", 1711700060]
  ]
}]
```

Columns are derived automatically from the first record. Column types are inferred: JSON numbers become `"number"`, booleans become `"boolean"`, everything else becomes `"string"`.

#### Time Series Format

Request:
```json
{
  "targets": [
    { "target": "app-siem/CostTracking", "type": "timeserie" }
  ],
  "range": {
    "from": "2024-01-01T00:00:00Z",
    "to": "2024-12-31T00:00:00Z"
  },
  "maxDataPoints": 100
}
```

Response:
```json
[{
  "target": "app-siem/CostTracking",
  "datapoints": [
    [0.045, 1711700000000],
    [0.120, 1711786400000]
  ]
}]
```

Each datapoint is `[value, timestamp_ms]`. The value is the first numeric field found in the record (skipping `id`, `timestamp`, `createdAt`, `updatedAt`, `time`). The timestamp is extracted from the first matching time field and normalized to milliseconds.

### Target Format

Targets follow the pattern `app_id/TableName`:

| Target | Description |
|--------|-------------|
| `yeti-telemetry/Log` | Telemetry log records |
| `yeti-telemetry/Span` | Telemetry span records |
| `yeti-telemetry/Metric` | Telemetry metric records |
| `app-siem/Event` | SIEM security events |
| `app-siem/CostTracking` | SIEM daily cost data |
| `app-cortex/Memory` | Cortex agent memories |
| `app-rate-limiter/RequestLog` | Rate limiter request logs |

Any yeti application with `@export` tables is queryable. Add a new app and its tables are available in Grafana immediately -- no configuration changes needed.

### Auto-Column Detection

When returning table format, app-grafana derives columns from the first record in the result set:

1. Iterates over all keys in the first JSON object
2. Inspects the value type of each key:
   - `Number` -> column type `"number"`
   - `Bool` -> column type `"boolean"`
   - Everything else -> column type `"string"`
3. Builds the `columns` array with `{"text": "fieldName", "type": "detectedType"}`
4. Extracts values from every record in key order to build `rows`

No schema mapping is required. Add a field to your yeti schema, and it appears as a new column in Grafana on the next query.

### Time Field Detection

For time series format, app-grafana scans each record for a timestamp using the following field names in priority order:

| Priority | Field Name | Description |
|----------|-----------|-------------|
| 1 | `timestamp` | Unix timestamp (seconds or milliseconds) |
| 2 | `createdAt` | Record creation time |
| 3 | `time` | Generic time field |
| 4 | `created_at` | Snake-case creation time |
| 5 | `updatedAt` | Last update time |

**Normalization:** Values less than 10 billion are assumed to be seconds and are multiplied by 1000 to convert to milliseconds. Values greater than or equal to 10 billion are assumed to be milliseconds already. Both numeric values and string-encoded numbers are supported.

The default time field can be overridden via the `timeField` setting in the DatasourceConfig table.

---

## Data Model

### DatasourceConfig Table

Runtime configuration for the datasource. A single record with `id: "default"` controls behavior:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `id` | ID! (primary key) | `"default"` | Configuration key |
| `baseUrl` | String | `http://127.0.0.1` | Yeti base URL for internal API calls |
| `allowedApps` | String | `""` (all apps) | JSON array of app_ids to expose in search (empty = all) |
| `timeField` | String | `"createdAt"` | Default time field name for timestamp extraction |

---

## Grafana Setup

### Prerequisites

- Grafana 7.0+ (any edition -- OSS, Enterprise, or Cloud)
- The **SimpleJSON** datasource plugin installed (bundled with most Grafana installations, or install via `grafana-cli plugins install grafana-simple-json-datasource`)
- A running yeti instance with app-grafana loaded

### Step-by-Step Configuration

**1. Open Data Sources**

Navigate to **Configuration** (gear icon in the left sidebar) > **Data Sources** > click **Add data source**.

**2. Select SimpleJSON**

Search for "SimpleJSON" in the plugin list and select it. If not found, install it first:
```bash
grafana-cli plugins install grafana-simple-json-datasource
sudo systemctl restart grafana-server
```

**3. Configure the connection**

| Setting | Value |
|---------|-------|
| **Name** | `Yeti` (or any descriptive name) |
| **URL** | `https://localhost/app-grafana` |
| **Access** | `Server (default)` |

**4. Configure authentication**

Under **Auth**:
- Check **Skip TLS Verify** if using self-signed development certificates (mkcert)

Under **Custom HTTP Headers**, click **Add header**:
- **Header:** `Authorization`
- **Value:** `Bearer <your-jwt-token>`

To obtain a JWT token:
```bash
curl -k -X POST https://localhost/yeti-auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "your-password"}'
```

**5. Save and test**

Click **Save & Test**. A green banner reading "Data source is working" confirms the connection.

**6. Create a dashboard panel**

- Click **+** > **Dashboard** > **Add new panel**
- Select the **Yeti** datasource
- Type a target in the metric field: `yeti-telemetry/Log`
- Choose **Table** format for log data, or **Time series** for numeric graphs
- Adjust the time range to match your data
- Click **Apply**

---

## Configuration

### DatasourceConfig (POST /app-grafana/api/DatasourceConfig)

Configure the datasource behavior at runtime by creating or updating the default config record:

```bash
curl -k -X POST https://localhost/app-grafana/api/DatasourceConfig \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "id": "default",
    "baseUrl": "http://127.0.0.1",
    "allowedApps": "[\"app-siem\", \"yeti-telemetry\"]",
    "timeField": "createdAt"
  }'
```

| Field | Example | Effect |
|-------|---------|--------|
| `baseUrl` | `http://127.0.0.1` | Where app-grafana fetches table data from. Change this to point at a different yeti instance. |
| `allowedApps` | `["app-siem"]` | Restricts `/search` results to only listed app IDs. Empty string or omitted = all apps visible. |
| `timeField` | `"timestamp"` | Overrides the default time field name for timestamp extraction in time series queries. |

### config.yaml

```yaml
name: "Grafana Datasource"
app_id: "app-grafana"
version: "0.1.0"
description: "Grafana SimpleJSON datasource — query yeti tables from Grafana dashboards"

schemas:
  path: schemas/schema.graphql

resources:
  path: resources/*.rs
  route: /api

auth:
  methods: [jwt, basic]
```

---

## Authentication

app-grafana uses yeti's built-in auth system configured in `config.yaml`:

- **JWT** and **Basic Auth** are enabled by default
- In **development mode**, all endpoints are accessible without authentication
- In **production mode**, all endpoints require a valid token

### Grafana-side authentication

Grafana's SimpleJSON plugin supports two authentication methods:

| Method | Configuration |
|--------|--------------|
| **Custom header** | Add `Authorization: Bearer <jwt-token>` as a custom HTTP header in the datasource settings |
| **Basic auth** | Check "Basic auth" in the datasource settings and enter yeti credentials |

### Token lifecycle

JWT tokens have an expiration time. For long-running Grafana dashboards:
- Use a service account with a long-lived token
- Or configure Basic Auth which does not expire
- Refresh tokens are available via `POST /yeti-auth/jwt_refresh`

---

## Project Structure

```
app-grafana/
  config.yaml              # App configuration
  schemas/
    schema.graphql         # DatasourceConfig table
  resources/
    search.rs              # Connection test + target listing
    query.rs               # Data fetching in SimpleJSON format
```

---

## Comparison

| | app-grafana | Traditional Grafana Plugin |
|---|---|---|
| **Languages** | Rust (yeti resource) | Go backend + TypeScript frontend |
| **Toolchain** | None -- compiles with yeti | Go compiler, Node.js, yarn, mage, Grafana plugin SDK |
| **Installation** | Copy folder, restart yeti | Build plugin, sign it, copy to Grafana plugins dir, restart Grafana |
| **Plugin signing** | Not required (SimpleJSON protocol) | Required for Grafana Cloud, optional for local |
| **Development cycle** | Edit .rs file, restart yeti | Rebuild Go + rebuild TS + restart Grafana |
| **New tables** | Automatically available via `app_id/TableName` | Requires code changes to add data sources |
| **Authentication** | Yeti built-in JWT/Basic | Custom implementation per plugin |
| **Deployment** | Single binary with yeti | Separate Go binary + JS bundle |
| **Lines of code** | ~160 lines of Rust (2 resources) | ~2000+ lines (Go + TypeScript + config) |
| **Dependencies** | yeti-sdk only | grafana-plugin-sdk-go, react, rxjs, webpack |
| **Time to first panel** | 5 minutes (clone + restart + add datasource) | Hours (scaffold + build + sign + configure) |

---

Built with [Yeti](https://yetirocks.com) | The Performance Platform for Agent-Driven Development
