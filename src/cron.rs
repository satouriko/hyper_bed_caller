use chrono::prelude::*;

#[derive(Debug)]
pub struct CronService {
  last_tick: DateTime<Local>,
}

impl CronService {
  pub fn new() -> CronService {
    CronService {
      last_tick: Local::now(),
    }
  }
  pub fn tick<T>(&mut self, f: T)
  where
    T: Fn(i64, i64),
  {
    let now = Local::now();
    f(self.last_tick.timestamp(), now.timestamp());
    self.last_tick = now;
  }
}
