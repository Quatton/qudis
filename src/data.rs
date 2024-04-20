use std::collections::HashMap;
use std::fs::{create_dir, File, OpenOptions};
use std::io::Result;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::sync::Mutex;

// use log::debug;

pub type Store = HashMap<String, String>;

pub struct AppData {
    pub store: Mutex<Store>,
}

impl AppData {
    pub fn new(store: Store) -> Self {
        Self {
            store: Mutex::new(store),
        }
    }
}

const WAL_DIR: &str = ".data";
const WAL_PATH: &str = ".data/wal.aof";

pub fn append_wal(command: &str) -> Result<()> {
    let path = Path::new(WAL_DIR);

    if !path.exists() {
        create_dir(path)?;
    }

    let mut file: File = OpenOptions::new()
        .append(true)
        .create(true)
        .open(WAL_PATH)?;

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
