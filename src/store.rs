use serde::{Deserialize, Serialize};
use serde_json;
use std::cell::{RefCell, RefMut};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {}

impl State {
    pub fn new() -> State {
        State {}
    }
}

pub struct Store {
    path: String,
    state: RefCell<State>,
}

impl Store {
    pub fn state(&self) -> RefMut<'_, State> {
        self.state.borrow_mut()
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
            state: RefCell::new(state.clone()),
        };
        store.save().unwrap();
        return store;
    }
}
