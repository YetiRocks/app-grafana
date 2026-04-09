use yeti_sdk::prelude::*;

// Grafana SimpleJSON /search endpoint.
//
// POST /app-grafana/search
//   Body: { "target": "" }
//   Returns: list of available metrics (app/table combinations)
//
// Also handles GET for connection test (Grafana "Test" button).
resource!(Search {
    name = "search",
    get(request, ctx) => {
        // Connection test — Grafana SimpleJSON calls GET on the datasource root
        ok(json!({"status": "ok", "message": "Yeti Grafana Datasource"}))
    },
    post(request, ctx) => {
        let body: Value = request.json()?;
        let filter = body["target"].as_str().unwrap_or("");
        let base_url = get_base_url(&ctx).await;

        let mut targets: Vec<String> = Vec::new();

        // Fetch app list from /health or /admin/apps
        if let Ok(resp) = fetch!(&format!("{}/health", base_url)).send() {
            if resp.ok() {
                let parsed: Value = serde_json::from_str(&resp.body).unwrap_or(json!({}));
                if let Some(apps) = parsed["applicationList"].as_array() {
                    for app in apps {
                        if let Some(app_id) = app.as_str() {
                            // For each app, the tables are queryable as "app_id/TableName"
                            targets.push(app_id.to_string());
                        }
                    }
                }
            }
        }

        // Filter by target prefix if provided
        let filtered: Vec<&String> = if filter.is_empty() {
            targets.iter().collect()
        } else {
            let lower = filter.to_lowercase();
            targets.iter().filter(|t| t.to_lowercase().contains(&lower)).collect()
        };

        ok(json!(filtered))
    }
});

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
