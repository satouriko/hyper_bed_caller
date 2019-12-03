use serde::{Deserialize, Serialize};
use serde_json;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs;
use std::sync::{Mutex, MutexGuard};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alarm {
  pub user_id: i64,
  pub chat_id: i64,
  pub cron: String,
  pub title: String,
  pub is_strict: bool,
  pub is_onceoff: bool,
  pub is_disabled: bool,
  pub is_pending: bool,
  pub is_informing: bool,
  pub strict_challenge: String,
  pub reschedule: i64,
}

impl Alarm {
  pub fn new<T>(user_id: i64, chat_id: i64, cron: T, title: T, is_strict: bool) -> Alarm
  where
    T: AsRef<str>,
  {
    Alarm {
      user_id,
      chat_id,
      cron: String::from(cron.as_ref()),
      title: String::from(title.as_ref()),
      is_strict,
      is_onceoff: false,
      is_disabled: false,
      is_pending: false,
      is_informing: false,
      strict_challenge: String::default(),
      reschedule: 0,
    }
  }
}

impl Display for Alarm {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
    f.write_str(&format!(
      "{}@{}://{}{}",
      self.user_id, self.chat_id, self.cron, self.title
    ))
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
  pub alarms: HashMap<i64, RefCell<Vec<Alarm>>>,
  pub timezone: HashMap<i64, String>,
  pub sleeping: HashMap<i64, RefCell<Vec<i64>>>,
  pub users: HashMap<i64, String>,
}

impl State {
  pub fn new() -> State {
    State {
      alarms: HashMap::new(),
      timezone: HashMap::new(),
      users: HashMap::new(),
      sleeping: HashMap::new(),
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
  pub fn new<T>(path: T) -> Store
  where
    T: AsRef<str>,
  {
    let contents = fs::read_to_string(path.as_ref());
    let state = match contents {
      Err(_) => State::new(),
      Ok(string) => serde_json::from_str(string.as_str()).expect("Bad JSON"),
    };
    let store = Store {
      path: String::from(path.as_ref()),
      state: Mutex::new(state.clone()),
    };
    store.save().unwrap();
    return store;
  }
}
