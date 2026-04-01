use yeti_sdk::prelude::*;

// Grafana SimpleJSON /query endpoint.
//
// POST /app-grafana/query
//   Body: {
//     "targets": [{ "target": "yeti-telemetry/Log", "type": "table" }],
//     "range": { "from": "2024-01-01T00:00:00Z", "to": "2024-01-02T00:00:00Z" },
//     "maxDataPoints": 100
//   }
//
// Returns time series or table data in Grafana SimpleJSON format.
// Target format: "app_id/TableName" (e.g., "yeti-telemetry/Log")
resource!(Query {
    name = "query",
    post(request, ctx) => {
        let body: Value = request.json()?;
        let targets = body["targets"].as_array()
            .ok_or_else(|| YetiError::Validation("missing targets array".into()))?;
        let max_points = body["maxDataPoints"].as_u64().unwrap_or(100) as usize;
        let base_url = get_base_url(&ctx).await;

        let mut results: Vec<Value> = Vec::new();

        for target_def in targets {
            let target = target_def["target"].as_str().unwrap_or("");
            let result_type = target_def["type"].as_str().unwrap_or("table");

            if target.is_empty() { continue; }

            // Parse target as "app_id/TableName"
            let parts: Vec<&str> = target.splitn(2, '/').collect();
            if parts.len() != 2 {
                continue;
            }
            let (app_id, table_name) = (parts[0], parts[1]);

            // Fetch records from the table via REST
            let url = format!("{}/{}/{}?limit={}", base_url, app_id, table_name, max_points);
            let resp = match fetch!(&url).send() {
                Ok(r) => r,
                Err(_) => continue,
            };

            if !resp.ok() { continue; }

            let records: Vec<Value> = serde_json::from_str(&resp.body).unwrap_or(vec![]);

            match result_type {
                "timeserie" | "timeseries" => {
                    // Time series format: { "target": "name", "datapoints": [[value, timestamp], ...] }
                    let datapoints: Vec<Value> = records.iter().filter_map(|r| {
                        let ts = extract_timestamp(r)?;
                        // Use the first numeric field as the value
                        let val = find_numeric_value(r)?;
                        Some(json!([val, ts]))
                    }).collect();

                    results.push(json!({
                        "target": target,
                        "datapoints": datapoints
                    }));
                },
                _ => {
                    // Table format: { "type": "table", "columns": [...], "rows": [...] }
                    if records.is_empty() {
                        results.push(json!({
                            "type": "table",
                            "columns": [],
                            "rows": []
                        }));
                        continue;
                    }

                    // Derive columns from first record
                    let first = &records[0];
                    let columns: Vec<Value> = first.as_object()
                        .map(|obj| obj.keys().map(|k| {
                            let col_type = match &first[k] {
                                Value::Number(_) => "number",
                                Value::Bool(_) => "boolean",
                                _ => "string",
                            };
                            json!({"text": k, "type": col_type})
                        }).collect())
                        .unwrap_or_default();

                    let column_keys: Vec<String> = first.as_object()
                        .map(|obj| obj.keys().cloned().collect())
                        .unwrap_or_default();

                    let rows: Vec<Value> = records.iter().map(|r| {
                        let row: Vec<Value> = column_keys.iter()
                            .map(|k| r.get(k).cloned().unwrap_or(Value::Null))
                            .collect();
                        json!(row)
                    }).collect();

                    results.push(json!({
                        "type": "table",
                        "columns": columns,
                        "rows": rows
                    }));
                }
            }
        }

        reply().json(json!(results))
    }
});

fn extract_timestamp(record: &Value) -> Option<u64> {
    // Try common time field names
    for field in &["timestamp", "createdAt", "time", "created_at", "updatedAt"] {
        if let Some(val) = record.get(*field) {
            if let Some(n) = val.as_u64() {
                // If < 10 billion, assume seconds; convert to ms
                return Some(if n < 10_000_000_000 { n * 1000 } else { n });
            }
            if let Some(s) = val.as_str() {
                if let Ok(n) = s.parse::<u64>() {
                    return Some(if n < 10_000_000_000 { n * 1000 } else { n });
                }
            }
        }
    }
    None
}

fn find_numeric_value(record: &Value) -> Option<f64> {
    if let Some(obj) = record.as_object() {
        for (key, val) in obj {
            // Skip known non-metric fields
            if matches!(key.as_str(), "id" | "timestamp" | "createdAt" | "updatedAt" | "time") {
                continue;
            }
            if let Some(n) = val.as_f64() {
                return Some(n);
            }
        }
    }
    None
}

async fn get_base_url(ctx: &ResourceParams) -> String {
    let config_table = ctx.get_table("DatasourceConfig");
    if let Ok(table) = config_table {
        if let Ok(Some(config)) = table.get("default").await {
            if let Some(url) = config["baseUrl"].as_str() {
                if !url.is_empty() {
                    return url.to_string();
                }
            }
        }
    }
    "http://127.0.0.1:9996".to_string()
}
