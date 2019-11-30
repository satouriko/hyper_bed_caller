use serde::{Deserialize, Serialize};
use serde_json;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::sync::{Mutex, MutexGuard};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alarm {
    pub user_id: i64,
    pub chat_id: i64,
    pub cron: String,
    pub title: String,
    pub is_strict: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub alarms: HashMap<i64, RefCell<Vec<Alarm>>>,
}

impl State {
    pub fn new() -> State {
        State {
            alarms: HashMap::new(),
        }
    }
}

pub struct Store {
    path: String,
    state: Mutex<State>,
}

impl Store {
    pub fn state(&self) -> MutexGuard<State> {
        self.state.lock().unwrap()
    }
    pub fn save(&self) -> Result<(), std::io::Error> {
        let json = serde_json::to_string(&*self.state()).expect("JSON serialize error");
        fs::write(self.path.as_str(), &json)?;
        Ok(())
    }
    pub fn new(path: &str) -> Store {
        let contents = fs::read_to_string(path);
        let state = match contents {
            Err(_) => State::new(),
            Ok(string) => serde_json::from_str(string.as_str()).expect("Bad JSON"),
        };
        let store = Store {
            path: String::from(path),
            state: Mutex::new(state.clone()),
        };
        store.save().unwrap();
        return store;
    }
}
