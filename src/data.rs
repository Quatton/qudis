use std::{collections::HashMap, sync::Mutex};

pub struct AppData {
    pub store: Mutex<HashMap<String, String>>,
}

impl AppData {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
        }
    }
}
