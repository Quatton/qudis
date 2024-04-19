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
    let path = path.into_inner();
    let path_split = path.split('/').collect::<Vec<&str>>();

    let key = path_split[0].to_string();
    let value = if path_split.len() > 1 {
        path_split[1].to_string()
    } else {
        let v = String::from_utf8(body.to_vec());

        match v {
            Ok(val) => val,
            Err(_) => {
                return HttpResponse::BadRequest().body("Invalid UTF-8 sequence");
            }
        }
    };

    data.store.lock().unwrap().insert(key, value);

    HttpResponse::Ok().body("OK".to_string())
}

#[post("/delete/{key}")]
async fn delete(data: web::Data<data::AppData>, path: web::Path<String>) -> impl Responder {
    let key = path.into_inner();
    data.store.lock().unwrap().remove(&key);

    HttpResponse::Ok().body("OK".to_string())
}

fn create_app_with_store(
    app_data: data::AppData,
) -> App<
    impl ServiceFactory<
        ServiceRequest,
        Config = (),
        Response = ServiceResponse<impl MessageBody>,
        Error = Error,
        InitError = (),
    >,
> {
    App::new()
        .app_data(web::Data::new(app_data))
        .service(get)
        .service(set)
        .service(delete)
        .wrap(Logger::new("%a %{User-Agent}i"))
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
    let app_data = data::AppData::new();

    create_app_with_store(app_data)
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
    async fn test_set_get_delete() {
        let app_data = data::AppData::new();

        let app = test::init_service(create_app_with_store(app_data)).await;
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
