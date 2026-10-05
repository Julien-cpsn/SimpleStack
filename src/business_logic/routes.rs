use std::collections::HashMap;
use std::time::Duration;
use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderValue, Method, StatusCode};
use axum::Router;
use axum::routing::{get, MethodRouter, post};
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;
use crate::business_logic::image::{upload_image, MAX_UPLOAD_SIZE};
use crate::business_logic::project::get_project_summary;
use crate::info;

const TARGET: &str = "route";

trait RouteMap {
    type Map;
    type Method;

    fn route_with_map(self, map: &mut Self::Map, path: &str, method_router: Self::Method) -> Self;
}

impl<S: Clone + Send + Sync + 'static> RouteMap for Router<S> {
    type Map = HashMap<String, MethodRouter<S>>;
    type Method = MethodRouter<S>;

    fn route_with_map(self, map: &mut Self::Map, path: &str, method_router: Self::Method) -> Self {
        map.insert(path.to_string(), method_router.clone());

        self.route(path, method_router)
    }
}

pub fn define_routes() -> Router {
    let mut routes = HashMap::new();

    let cors = CorsLayer::new()
        .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST]);

    let router = Router::<()>::new()
        .route_with_map(&mut routes, "/", get(|| async { "Hello, World!" }))
        .route_with_map(&mut routes, "/project-summary", get(get_project_summary))
        .route_with_map(&mut routes, "/upload-image", post(upload_image))
        .layer(cors)
        .layer(DefaultBodyLimit::disable())
        .layer(DefaultBodyLimit::max(MAX_UPLOAD_SIZE as usize))
        .layer(TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, Duration::from_secs(60)));

    for route in routes.keys() {
        info!("Registering route: {}", route);
    }

    router
}