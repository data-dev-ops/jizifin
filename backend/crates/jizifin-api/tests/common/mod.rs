use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::response::Response;
use http_body_util::BodyExt;
use jizifin_api::{AppState, router};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use tower::ServiceExt;

pub const TEST_PROOF: &str = "test-auth-proof-aes-gcm-base64url";

#[allow(dead_code)]
pub struct TestClient {
    pub state: AppState,
    pub token: String,
}

#[allow(dead_code)]
impl TestClient {
    pub async fn uninitialized() -> Self {
        let state = AppState::in_memory().unwrap();
        Self { state, token: String::new() }
    }

    pub async fn authenticated() -> Self {
        let state = AppState::in_memory().unwrap();
        let app = router(state.clone());
        let body = serde_json::json!({ "proof": TEST_PROOF });
        let req = Request::post("/auth/init")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let val: Value = json_body(resp).await;
        let token = val["token"].as_str().unwrap().to_string();
        Self { state, token }
    }

    pub async fn request(&self, req: Request<Body>) -> Response {
        let app = router(self.state.clone());
        app.oneshot(req).await.unwrap()
    }

    pub async fn get(&self, uri: &str) -> Response {
        let mut builder = Request::get(uri);
        if !self.token.is_empty() {
            builder = builder.header("authorization", format!("Bearer {}", self.token));
        }
        self.request(builder.body(Body::empty()).unwrap()).await
    }

    pub async fn post_json<T: Serialize>(&self, uri: &str, payload: &T) -> Response {
        let mut builder = Request::post(uri).header("content-type", "application/json");
        if !self.token.is_empty() {
            builder = builder.header("authorization", format!("Bearer {}", self.token));
        }
        let data = serde_json::to_vec(payload).unwrap();
        self.request(builder.body(Body::from(data)).unwrap()).await
    }

    pub async fn put_json<T: Serialize>(&self, uri: &str, payload: &T) -> Response {
        let mut builder = Request::put(uri).header("content-type", "application/json");
        if !self.token.is_empty() {
            builder = builder.header("authorization", format!("Bearer {}", self.token));
        }
        let data = serde_json::to_vec(payload).unwrap();
        self.request(builder.body(Body::from(data)).unwrap()).await
    }

    pub async fn delete(&self, uri: &str) -> Response {
        let mut builder = Request::delete(uri);
        if !self.token.is_empty() {
            builder = builder.header("authorization", format!("Bearer {}", self.token));
        }
        self.request(builder.body(Body::empty()).unwrap()).await
    }
}

pub async fn json_body<T: DeserializeOwned>(resp: Response) -> T {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}
