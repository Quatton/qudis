mod data;

use actix_web::{
    body::MessageBody,
    dev::{ServiceFactory, ServiceRequest, ServiceResponse},
    get,
    middleware::Logger,
    post, web, App, Error, HttpResponse, HttpServer, Responder,
};

#[get("/get/{key}")]
async fn get(data: web::Data<data::AppData>, path: web::Path<String>) -> impl Responder {
    let key = path.into_inner();
    match data.store.lock().unwrap().get(&key) {
        Some(value) => HttpResponse::Ok().body(value.to_string()),
        None => HttpResponse::Ok().body("NOT FOUND"),
    }
}

#[post("/set/{tail:.*}")]
async fn set(
    data: web::Data<data::AppData>,
    path: web::Path<String>,
    body: web::Bytes,
) -> impl Responder {
    let key = path.into_inner();
    let value = String::from_utf8(body.to_vec()).unwrap();
    data.store.lock().unwrap().insert(key, value);
    HttpResponse::Ok()
}

fn create_app() -> App<
    impl ServiceFactory<
        ServiceRequest,
        Config = (),
        Response = ServiceResponse<impl MessageBody>,
        Error = Error,
        InitError = (),
    >,
> {
    let store = data::AppData::new();

    App::new()
        .app_data(web::Data::new(store))
        .service(get)
        .service(set)
        .wrap(Logger::new("%a %{User-Agent}i"))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    HttpServer::new(create_app)
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use actix_web::{http::Method, test};

    use super::*;

    #[actix_web::test]
    async fn test_get_key() {
        let app = test::init_service(create_app()).await;
        let req = test::TestRequest::with_uri("/get/test").to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 200);
        assert_eq!(test::read_body(resp).await, "NOT FOUND");
    }

    #[actix_web::test]
    async fn test_set_key() {
        let app = test::init_service(create_app()).await;
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
    }
}
