mod app;
mod data;

use std::sync::Arc;

use actix_web::rt::signal::unix::{signal, SignalKind};
use actix_web::HttpServer;
use app::create_app;
use aws_sdk_s3::Client;
use data::{load_wal, AppData};
use log::info;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let shared_config = aws_config::from_env().load().await;

    let db = load_wal()?;
    let client = Client::new(&shared_config);
    let app_data = Arc::new(AppData::new(db, client));

    if std::env::var("DEV").unwrap_or("false".to_string()) == "false"
        && app_data.download_wal().await.is_ok()
    {
        info!("Downloaded WAL")
    }

    let term_app_data = app_data.clone();

    let scheduler_data = app_data.clone();

    let server = HttpServer::new(move || create_app(app_data.clone()))
        .bind(("0.0.0.0", 8080))?
        .disable_signals()
        .run();

    let term_handler = server.handle().clone();

    // This scheduler will be used to upload the WAL file to S3
    tokio::spawn(async move {
        scheduler_data.start_scheduler().await;
    });

    tokio::spawn(async move {
        let mut term = signal(SignalKind::terminate()).unwrap();
        let mut int = signal(SignalKind::interrupt()).unwrap();

        tokio::select! {
            _ = term.recv() => {
                info!("Forcing shutdown");
            }
            _ = int.recv() => {
                info!("Forcing shutdown");
            }
        }

        let _ = term_app_data.upload_wal().await;

        term_handler.stop(true).await
    });

    server.await
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use actix_web::{http::Method, test};

    use super::*;

    #[actix_web::test]
    async fn test_set_get_delete() {
        let store = HashMap::new();
        let app_data = Arc::new(data::AppData {
            store: Mutex::new(store),
            client: None,
        });

        let app = test::init_service(create_app(app_data)).await;
        let req = test::TestRequest::with_uri("/set/test")
            .method(Method::POST)
            .set_payload("value")
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 200);

        let req = test::TestRequest::with_uri("/get/test").to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 200);
        assert_eq!(test::read_body(resp).await, "value");

        let req = test::TestRequest::with_uri("/delete/test")
            .method(Method::POST)
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 200);

        let req = test::TestRequest::with_uri("/get/test").to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 200);

        assert_eq!(test::read_body(resp).await, "NOT FOUND");
    }
}
