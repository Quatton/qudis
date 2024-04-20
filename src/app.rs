use crate::data::{append_wal, AppData};
use std::sync::Arc;

use actix_web::HttpResponse;
use actix_web::{
    body::MessageBody,
    dev::{ServiceFactory, ServiceRequest, ServiceResponse},
    get,
    middleware::Logger,
    post, web, App, Error, Responder,
};
use log::info;

#[get("/get/{key}")]
async fn get(data: web::Data<Arc<AppData>>, path: web::Path<String>) -> impl Responder {
    let key = path.into_inner();
    match data.store.lock().unwrap().get(&key) {
        Some(value) => HttpResponse::Ok().body(value.to_string()),
        None => HttpResponse::Ok().body("NOT FOUND"),
    }
}

#[post("/set/{tail:.*}")]
async fn set(
    data: web::Data<Arc<AppData>>,
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

    data.store
        .lock()
        .unwrap()
        .insert(key.clone(), value.clone());

    info!("SET {} {}", key, value);
    append_wal(&format!("SET {} {}", key, value)).expect("Failed to backup in log");

    HttpResponse::Ok().body("OK".to_string())
}

#[post("/delete/{key}")]
async fn delete(data: web::Data<Arc<AppData>>, path: web::Path<String>) -> impl Responder {
    let key = path.into_inner();
    data.store.lock().unwrap().remove(&key);

    HttpResponse::Ok().body("OK".to_string())
}

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

pub fn create_app(
    app_data: Arc<AppData>,
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
        .service(index)
        .service(get)
        .service(set)
        .service(delete)
        .wrap(Logger::new("%a %{User-Agent}i"))
}

pub fn create_test_app(
    app_data: AppData,
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
        .service(index)
        .service(get)
        .service(set)
        .service(delete)
        .wrap(Logger::new("%a %{User-Agent}i"))
}
