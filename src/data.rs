use std::{collections::HashMap, sync::Mutex};

pub type Store = Mutex<HashMap<String, String>>;

pub struct AppData {
    pub store: Store,
}

impl AppData {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
        }
    }
}
