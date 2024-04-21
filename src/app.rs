use crate::data::{append_wal, AppData};
use std::sync::Arc;

use actix_web::error::{ErrorForbidden, ErrorUnauthorized};
use actix_web::HttpResponse;
use actix_web::{
    body::MessageBody,
    dev::{ServiceFactory, ServiceRequest, ServiceResponse},
    get,
    middleware::Logger,
    post, web, App, Error, Responder,
};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use actix_web_httpauth::middleware::HttpAuthentication;
use jsonwebtoken::{encode, DecodingKey, EncodingKey, Header};
use log::info;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[get("/get/{key}")]
async fn get(
    data: web::Data<Arc<AppData>>,
    path: web::Path<String>,
    auth: web::Data<Auth>,
    credentials: BearerAuth,
) -> impl Responder {
    match validate_token(credentials, &auth.secret) {
        Ok(user_id) => {
            let key = format!("{}:{}", user_id, path.into_inner());
            match data.store.lock().unwrap().get(&key) {
                Some(value) => HttpResponse::Ok().body(value.to_string()),
                None => HttpResponse::Ok().body("NOT FOUND"),
            }
        }
        _ => HttpResponse::Unauthorized().finish(),
    }
}

#[post("/set/{tail:.*}")]
async fn set(
    data: web::Data<Arc<AppData>>,
    path: web::Path<String>,
    auth: web::Data<Auth>,
    credentials: BearerAuth,
    body: web::Bytes,
) -> impl Responder {
    match validate_token(credentials, &auth.secret) {
        Ok(user_id) => {
            let path = path.into_inner();
            let path_split = path.split('/').collect::<Vec<&str>>();

            let key = format!("{}:{}", user_id, path_split[0]);
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
        _ => HttpResponse::Unauthorized().finish(),
    }
}

#[post("/delete/{key}")]
async fn delete(
    data: web::Data<Arc<AppData>>,
    path: web::Path<String>,
    auth: web::Data<Auth>,
    credentials: BearerAuth,
) -> impl Responder {
    match validate_token(credentials, &auth.secret) {
        Ok(user_id) => {
            let key = format!("{}:{}", user_id, path.into_inner());
            data.store.lock().unwrap().remove(&key);

            HttpResponse::Ok().body("OK".to_string())
        }
        _ => HttpResponse::Unauthorized().finish(),
    }
}

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    username: String,
    exp: i64,
}

#[derive(Deserialize)]
struct Info {
    username: String,
}

fn create_user_id(username: &str) -> String {
    let mut hasher = Sha256::default();
    hasher.update(username.as_bytes());
    let result = hasher.finalize();
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, result)
}

#[get("/auth/issue-token")]
async fn issue_token(info: web::Query<Info>, auth: web::Data<Auth>) -> impl Responder {
    let secret = auth.secret.clone();
    let user_id = create_user_id(&info.username);

    let claims = Claims {
        sub: user_id.clone(),
        username: info.username.clone(),

        // 4 hours in the future
        exp: (chrono::Utc::now() + chrono::Duration::hours(4)).timestamp(),
    };

    let token = match encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    ) {
        Ok(t) => t,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    HttpResponse::Ok().body(token)
}

fn validate_token(credentials: BearerAuth, secret: &str) -> Result<String, Error> {
    match jsonwebtoken::decode::<Claims>(
        credentials.token(),
        &DecodingKey::from_secret(secret.as_ref()),
        &jsonwebtoken::Validation::default(),
    ) {
        Ok(token) => {
            let user_id = create_user_id(&token.claims.username);

            // Check if the user_id is the same as the sub

            if user_id == token.claims.sub {
                return Ok(user_id);
            }

            Err(ErrorForbidden(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Forbidden",
            )))
        }
        Err(_) => Err(ErrorForbidden(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Forbidden",
        ))),
    }
}

async fn validator(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");

    if credentials.token().is_empty() {
        return Err((
            ErrorUnauthorized(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unauthorized",
            )),
            req,
        ));
    }

    match validate_token(credentials, &secret) {
        Ok(_) => Ok(req),
        Err(e) => Err((e, req)),
    }
}

struct Auth {
    pub secret: String,
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
        .app_data(web::Data::new(Auth {
            secret: std::env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
        }))
        .service(index)
        .service(issue_token)
        .service(
            web::scope("")
                .wrap(HttpAuthentication::bearer(validator))
                .service(get)
                .service(set)
                .service(delete),
        )
        .wrap(Logger::new("%a %{User-Agent}i"))
}
