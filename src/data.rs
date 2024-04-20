use std::collections::HashMap;
use std::fs::{create_dir, File, OpenOptions};
use std::io::Result;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::sync::Mutex;

use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{BucketLocationConstraint, CreateBucketConfiguration};
use aws_sdk_s3::Client;
use chrono::Utc;
use log::{error, info, warn};
use tokio_schedule::{every, Job};

// use log::debug;

pub type Store = HashMap<String, String>;

pub struct AppData {
    pub store: Mutex<Store>,
    pub client: Option<Client>,
}

impl AppData {
    pub fn new(store: Store, client: Client) -> Self {
        Self {
            store: Mutex::new(store),
            client: Some(client),
        }
    }

    pub async fn start_scheduler(&self) {
        let schedule = every(1).hour().in_timezone(&Utc).perform(|| async {
            info!("Backing up WAL");
            let _ = self.upload_wal().await;
        });
        schedule.await
    }

    pub async fn upload_wal(&self) -> Result<()> {
        if let Some(client) = &self.client {
            let body = ByteStream::from_path(WAL_PATH).await;

            if !&self.is_bucket_ready().await {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Bucket is not ready",
                ));
            }

            match body {
                Ok(b) => {
                    let resp = client
                        .put_object()
                        .bucket(BUCKET_NAME)
                        .key("wal.aof")
                        .body(b)
                        .send()
                        .await;

                    match resp {
                        Ok(_) => {
                            info!("WAL uploaded");
                            Ok(())
                        }
                        Err(SdkError::ServiceError(err)) => {
                            let http = err.raw();
                            error!("Cannot upload WAL: {}", http.status());
                            Err(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                "Cannot upload WAL",
                            ))
                        }
                        Err(_) => {
                            error!("Cannot upload WAL");
                            Err(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                "Cannot upload WAL",
                            ))
                        }
                    }
                }
                Err(err) => {
                    error!("Cannot read WAL");
                    Err(err.into())
                }
            }
        } else {
            warn!("Client is not available");
            Ok(())
        }
    }

    pub async fn download_wal(&self) -> Result<()> {
        if let Some(client) = &self.client {
            let resp = client
                .get_object()
                .bucket(BUCKET_NAME)
                .key("wal.aof")
                .send()
                .await;

            match create_dir(WAL_DIR) {
                Ok(_) => (),
                Err(err) => {
                    if err.kind() != std::io::ErrorKind::AlreadyExists {
                        error!("Cannot create directory");
                        return Err(err);
                    }
                }
            };

            if let Ok(mut file) = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(WAL_PATH)
            {
                match resp {
                    Ok(mut resp) => {
                        while let Some(chunk) = resp.body.try_next().await? {
                            file.write_all(&chunk)?;
                        }

                        info!("WAL downloaded");
                        Ok(())
                    }
                    Err(SdkError::ServiceError(err)) => {
                        let http = err.raw();
                        error!("Cannot download WAL: {}", http.status());
                        Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Cannot download WAL",
                        ))
                    }
                    Err(_) => {
                        error!("Cannot download WAL");
                        Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Cannot download WAL",
                        ))
                    }
                }
            } else {
                error!("Cannot create file");
                Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Cannot create file",
                ))
            }
        } else {
            warn!("Client is not available");
            Ok(())
        }
    }

    pub async fn is_bucket_ready(&self) -> bool {
        if let Some(client) = &self.client {
            match client.head_bucket().bucket(BUCKET_NAME).send().await {
                Ok(_) => true,
                Err(SdkError::ServiceError(err)) => {
                    let http = err.raw();
                    match http.status().as_u16() {
                        404 => {
                            // let's create a bucket!
                            let constraint = BucketLocationConstraint::from("ap-northeast-1");

                            let cfg = CreateBucketConfiguration::builder()
                                .location_constraint(constraint)
                                .build();

                            let resp = client
                                .create_bucket()
                                .create_bucket_configuration(cfg)
                                .bucket(BUCKET_NAME)
                                .send()
                                .await;

                            match resp {
                                Ok(_) => true,
                                Err(SdkError::ServiceError(err)) => {
                                    let http = err.raw();

                                    error!("{}", http.status());
                                    false
                                }
                                Err(err) => {
                                    error!("{}", err);
                                    false
                                }
                            }
                        }
                        code => {
                            error!("{}", code);
                            false
                        }
                    }
                }
                Err(err) => {
                    error!("{}", err);
                    false
                }
            }
        } else {
            warn!("Client is not available");
            false
        }
    }
}

const WAL_DIR: &str = ".data";
const WAL_PATH: &str = ".data/wal.aof";
const BUCKET_NAME: &str = "qudis-wal";

pub fn get_wal_file() -> Result<File> {
    let path = Path::new(WAL_PATH);

    if !path.exists() {
        create_dir(WAL_DIR)?;
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
    } else {
        OpenOptions::new().append(true).open(path)
    }
}

pub fn append_wal(command: &str) -> Result<()> {
    let mut file = get_wal_file()?;

    writeln!(file, "{}", command)?;
    Ok(())
}

pub fn load_wal() -> Result<HashMap<String, String>> {
    let path = Path::new(WAL_PATH);

    let mut db = HashMap::new();

    if path.exists() {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        reader.lines().for_each(|line| {
            if let Ok(it) = line {
                let parts: Vec<&str> = it.splitn(3, ' ').collect();

                // debug!("{}", &format!("{:?}", parts));

                match parts[..] {
                    ["SET", key, value] => {
                        db.insert(key.to_string(), value.to_string());
                    }
                    ["DELETE", key] => {
                        db.remove(key);
                    }
                    _ => {}
                }
            };
        });
    }

    Ok(db)
}
