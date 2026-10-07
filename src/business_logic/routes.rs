use std::collections::HashMap;
use std::time::Duration;
use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderValue, Method, StatusCode};
use axum::{middleware, Router};
use axum::routing::{get, MethodRouter, post};
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;
use crate::business_logic::auth::{create_user, login, logout, me, require_admin, require_auth};
use crate::business_logic::image::{upload_image, MAX_UPLOAD_SIZE, list_user_images};
use crate::business_logic::project::get_project_summary;
use crate::business_logic::vm::{create_vm, delete_vm, list_user_vms};
use crate::info;
use crate::server::ServerState;

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

pub fn define_routes(server_state: ServerState) -> Router {
    let mut routes = HashMap::new();

    let cors = CorsLayer::new()
        .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST]);

    let admin = Router::new()
        .route_with_map(&mut routes, "/new-user", post(create_user))
        .route_with_map(&mut routes, "/projects-summary", get(get_project_summary))
        .route_layer(middleware::from_fn(require_admin));

    let protected = Router::new()
        .route_with_map(&mut routes, "/me", get(me))
        .route_with_map(&mut routes, "/logout", post(logout))
        .route_with_map(&mut routes, "/upload-image", post(upload_image))
        //.route_with_map(&mut routes, "/delete-image/{uuid}", get(delete_image))
        .route_with_map(&mut routes, "/my-images", get(list_user_images))
        .route_with_map(&mut routes, "/create-vm", post(create_vm))
        .route_with_map(&mut routes, "/delete-vm/{name}", get(delete_vm))
        .route_with_map(&mut routes, "/my-vms", get(list_user_vms))
        .merge(admin)
        .route_layer(middleware::from_fn_with_state(server_state.clone(), require_auth))
        .layer(cors)
        .layer(DefaultBodyLimit::disable())
        .layer(DefaultBodyLimit::max(MAX_UPLOAD_SIZE as usize))
        .layer(TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, Duration::from_secs(60)));

    let router = Router::new()
        .route_with_map(&mut routes, "/login", post(login))
        .route_with_map(&mut routes, "/", get(|| async { "Hello, World!" }))
        .merge(protected)
        .with_state(server_state);

    for route in routes.keys() {
        info!("Registering route: {}", route);
    }

    router
}