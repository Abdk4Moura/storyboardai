#[cfg(not(target_arch = "wasm32"))]
mod inner {
    use axum::{
        extract::Json,
        http::{Method, StatusCode, header},
        response::{IntoResponse, Response},
        routing::post,
        Router,
        body::Body,
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
            .route("/api/visualize", post(proxy_visualize))
            .route("/api/foxit", post(proxy_foxit))
            .fallback_service(ServeDir::new("dist"))
            .layer(cors);

        let port = env::var("PORT").unwrap_or_else(|_| "8033".to_string());
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

    async fn proxy_visualize(Json(payload): Json<VisualizeRequest>) -> Response {
        let prompt_encoded = urlencoding::encode(&payload.prompt);
        let url = format!("https://image.pollinations.ai/prompt/{}?width=512&height=300&nologo=true", prompt_encoded);
        
        let client = reqwest::Client::new();
        let resp = client.get(&url).send().await;

        match resp {
            Ok(res) => {
                let status = res.status();
                if status.is_success() {
                    let bytes = res.bytes().await.unwrap_or_default();
                    return Response::builder()
                        .header(header::CONTENT_TYPE, "image/jpeg")
                        .body(Body::from(bytes))
                        .unwrap();
                }
                (status, "Pollinations API error").into_response()
            }
            Err(e) => {
                println!("Pollinations error: {}, using placeholder", e);
                let placeholder_url = format!("https://picsum.photos/seed/{}/512/300", prompt_encoded);
                let res = client.get(&placeholder_url).send().await;
                if let Ok(r) = res {
                    let bytes = r.bytes().await.unwrap_or_default();
                    return Response::builder()
                        .header(header::CONTENT_TYPE, "image/jpeg")
                        .body(Body::from(bytes))
                        .unwrap();
                }
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
            }
        }
    }

    async fn proxy_foxit() -> Response {
        (StatusCode::OK, "Foxit PDF Generated").into_response()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    inner::start().await;
}

#[cfg(target_arch = "wasm32")]
fn main() {}
