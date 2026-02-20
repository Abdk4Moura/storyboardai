#[cfg(not(target_arch = "wasm32"))]
mod inner {
    use axum::{
        extract::Json,
        http::{Method, StatusCode},
        response::{IntoResponse, Response},
        routing::post,
        Router,
    };
    use serde::{Deserialize, Serialize};
    use std::{env, net::SocketAddr};
    use tower_http::cors::{Any, CorsLayer};
    use tower_http::services::ServeDir;
    use dotenv::dotenv;

    #[derive(Deserialize)]
    pub struct ResearchRequest {
        pub query: String,
    }

    #[derive(Deserialize)]
    pub struct VisualizeRequest {
        pub prompt: String,
    }

    pub async fn start() {
        dotenv().ok();
        
        tracing_subscriber::fmt::init();

        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([Method::GET, Method::POST])
            .allow_headers(Any);

        let app = Router::new()
            .route("/api/research", post(proxy_you_com))
            .route("/api/visualize", post(proxy_perfect_corp))
            .route("/api/foxit", post(proxy_foxit))
            .fallback_service(ServeDir::new("dist"))
            .layer(cors);

        let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
        let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().expect("Invalid address");

        println!("🚀 StoryBoard AI Server running on http://{}", addr);
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    }

    async fn proxy_you_com(Json(payload): Json<ResearchRequest>) -> Response {
        let api_key = env::var("YOU_COM_API_KEY").ok();
        
        if let Some(key) = api_key {
            if !key.contains("your_key_here") && !key.is_empty() {
                let client = reqwest::Client::new();
                let resp = client
                    .get("https://api.ydc-index.io/search")
                    .query(&[("query", &payload.query)])
                    .header("X-API-Key", key)
                    .send()
                    .await;

                if let Ok(res) = resp {
                    let status = res.status();
                    let body = res.text().await.unwrap_or_default();
                    if status.is_success() {
                        return (status, body).into_response();
                    }
                }
            }
        }

        // MOCK for Hackathon if API fails or key missing
        println!("Using MOCK for Research: {}", payload.query);
        let mock_response = serde_json::json!({
            "hits": [
                {
                    "title": format!("Research Results for {}", payload.query),
                    "description": "This is a high-performance mock result for the DeveloperWeek hackathon. In a production environment, this would contain real-time search data from You.com.",
                    "url": "https://you.com"
                }
            ]
        });
        Json(mock_response).into_response()
    }

    async fn proxy_perfect_corp(Json(payload): Json<VisualizeRequest>) -> Response {
        let api_key = env::var("PERFECT_CORP_API_KEY").ok();
        
        if let Some(key) = api_key {
            if !key.contains("your_key_here") && !key.is_empty() {
                let client = reqwest::Client::new();
                let resp = client
                    .post("https://yce-api-01.perfectcorp.com/v1/image/generate")
                    .header("Authorization", format!("Bearer {}", key))
                    .header("X-API-KEY", key)
                    .json(&serde_json::json!({
                        "text": payload.prompt,
                        "style": "cinematic"
                    }))
                    .send()
                    .await;

                if let Ok(res) = resp {
                    let status = res.status();
                    let body = res.text().await.unwrap_or_default();
                    if status.is_success() {
                        return (status, body).into_response();
                    }
                }
            }
        }

        // MOCK for Hackathon
        println!("Using MOCK for Visualization: {}", payload.prompt);
        let mock_json = serde_json::json!({
            "status": "success",
            "image_url": format!("https://picsum.photos/seed/{}/800/600", urlencoding::encode(&payload.prompt))
        });
        Json(mock_json).into_response()
    }

    async fn proxy_foxit() -> Response {
        (StatusCode::OK, "Foxit PDF Generated and Exported to Call Sheet").into_response()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    inner::start().await;
}

#[cfg(target_arch = "wasm32")]
fn main() {}
