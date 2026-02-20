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

    #[derive(Deserialize)]
    pub struct AgnosticAIRequest {
        pub model: String,
        pub prompt: String,
    }

    #[derive(Deserialize)]
    pub struct FoxitRequest {
        pub all_node_text: String,
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
            .route("/api/agnostic-ai", post(proxy_agnostic_ai))
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

        // MOCK for Hackathon
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
            Err(_) => {
                let placeholder_url = format!("https://picsum.photos/seed/{}/512/300", prompt_encoded);
                let res = client.get(&placeholder_url).send().await;
                if let Ok(r) = res {
                    let bytes = r.bytes().await.unwrap_or_default();
                    return Response::builder()
                        .header(header::CONTENT_TYPE, "image/jpeg")
                        .body(Body::from(bytes))
                        .unwrap();
                }
                (StatusCode::INTERNAL_SERVER_ERROR, "Fallback failed").into_response()
            }
        }
    }

    async fn proxy_agnostic_ai(Json(payload): Json<AgnosticAIRequest>) -> Response {
        let api_key = env::var("OPENROUTER_API_KEY").ok();
        if let Some(key) = api_key {
            if !key.is_empty() && !key.contains("your_") {
                let client = reqwest::Client::new();
                let body = serde_json::json!({
                    "model": payload.model,
                    "messages": [{ "role": "user", "content": payload.prompt }]
                });
                let resp = client.post("https://openrouter.ai/api/v1/chat/completions")
                    .header("Authorization", format!("Bearer {}", key))
                    .header("HTTP-Referer", "http://localhost:8033")
                    .json(&body)
                    .send()
                    .await;
                if let Ok(res) = resp {
                    let status = res.status();
                    if status.is_success() {
                        if let Ok(json) = res.json::<serde_json::Value>().await {
                            if let Some(content) = json["choices"][0]["message"]["content"].as_str() {
                                return content.to_string().into_response();
                            }
                        }
                    }
                }
            }
        }
        format!("🎬 MOCK SCENE\n\nModel: {}\n\nBased on: {}\n\nFADE OUT.", payload.model, payload.prompt).into_response()
    }

    async fn proxy_foxit(Json(payload): Json<FoxitRequest>) -> Response {
        let client_id = "foxit_1mg1IazuGGpb3NWQ";
        let client_secret = "ZhhY5qqXIC3S1JBiqN8nE5zKWE48IBLR";
        
        let html_content = format!(
            "<!DOCTYPE html><html><head><title>StoryBoard AI Report</title></head><body><h1>StoryBoard AI - Generated Report</h1><hr><h2>Nodes:</h2><pre>{}</pre></body></html>",
            payload.all_node_text
        );

        let client = reqwest::Client::new();
        
        // STEP 1: UPLOAD
        let form = reqwest::multipart::Form::new()
            .part("file", reqwest::multipart::Part::bytes(html_content.into_bytes()).file_name("report.html"));

        let upload_resp = client.post("https://na1.fusion.foxit.com/pdf-services/api/documents/upload")
            .header("client_id", client_id)
            .header("client_secret", client_secret)
            .multipart(form)
            .send()
            .await;

        match upload_resp {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(doc_id) = json["documentId"].as_str() {
                        // STEP 2: CONVERT
                        let task_resp = client.post("https://na1.fusion.foxit.com/pdf-services/api/documents/create/pdf-from-html")
                            .header("client_id", client_id)
                            .header("client_secret", client_secret)
                            .json(&serde_json::json!({ "documentId": doc_id }))
                            .send()
                            .await;
                        
                        match task_resp {
                            Ok(t_resp) if t_resp.status().is_success() => {
                                return (StatusCode::OK, "PDF generation started successfully!").into_response();
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }

        // Fallback for Hackathon
        (StatusCode::OK, "✅ Foxit PDF Report Task Created (Mock Success)").into_response()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    inner::start().await;
}

#[cfg(target_arch = "wasm32")]
fn main() {}
