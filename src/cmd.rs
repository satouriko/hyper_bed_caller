extern crate cron;
use crate::fmt::*;
use crate::store::{Alarm, Store};
use chrono::{self, prelude::*};
use cron::Schedule;
use rtdlib::types::InputMessageContent;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct Command<'a> {
  cmd: &'a str,
  arg: &'a str,
}

impl Command<'_> {
  pub fn cmd(&self) -> &str {
    self.cmd
  }
  pub fn arg(&self) -> &str {
    self.arg
  }
}

pub fn parse_command_msg(input: &str) -> Command {
  let input = input.trim();
  let first_space = input.find(char::is_whitespace);
  match first_space {
    Some(first_space) => Command {
      cmd: &input[..first_space].trim(),
      arg: &input[(first_space + 1)..].trim(),
    },
    None => Command {
      cmd: input,
      arg: "",
    },
  }
}

#[derive(Debug, Clone)]
pub struct CronArgs<'a> {
  cron: String,
  title: &'a str,
}

impl CronArgs<'_> {
  pub fn cron(&self) -> &str {
    self.cron.as_str()
  }
  pub fn title(&self) -> &str {
    self.title
  }
}

fn test_cron<T>(input: T) -> Result<String, &'static str>
where
  T: AsRef<str>,
{
  let string = format!("0 {}", input.as_ref());
  let test = Schedule::from_str(string.as_str());
  match test {
    Ok(_) => Ok(string),
    Err(_) => Err("Bad cron expression"),
  }
}

fn test_time_str<T, Z>(input: T, tz: &Z) -> Result<String, &'static str>
where
  T: AsRef<str>,
  Z: TimeZone,
  Z::Offset: Display,
{
  let input = input.as_ref().trim();
  let first_space = input.find(char::is_whitespace);
  let time_str = match first_space {
    Some(first_space) => &input[..first_space],
    None => input,
  };
  let day_str = match first_space {
    Some(first_space) => &input[(first_space + 1)..],
    None => "once",
  };
  let first_colon = time_str.find(':');
  if let None = first_colon {
    return Err("Bad time string: Missing selecolon");
  };
  let first_colon = first_colon.unwrap();
  let h = (&time_str[..first_colon]).parse::<i32>();
  if let Err(_) = h {
    return Err("Bad time string: Hour must be an integer");
  };
  let h = h.unwrap();
  if h < 0 || h > 23 {
    return Err("Bad time string: Hour must between 0-23");
  };
  let m = (&time_str[(first_colon + 1)..]).parse::<i32>();
  if let Err(_) = m {
    return Err("Bad time string: Minute must be an integer");
  };
  let m = m.unwrap();
  if m < 0 || m > 59 {
    return Err("Bad time string: Minute must between 0-59");
  };
  match day_str {
    "once" => {
      let now = chrono::Local::now().with_timezone(tz);
      let fmt_str = format!("%Y-%m-%d {}:{}:00 %z", h, m);
      let today_alarm_str = now.format(fmt_str.as_str()).to_string();
      let today_alarm_time =
        chrono::DateTime::parse_from_str(today_alarm_str.as_str(), "%Y-%m-%d %H:%M:%S %z")
          .expect("Error parsing time string");
      if now.timestamp() > today_alarm_time.timestamp() {
        let tomorrow = now + chrono::Duration::days(1);
        Ok(format!(
          "{} {} {} {} * {}",
          m,
          h,
          tomorrow.day(),
          tomorrow.month(),
          tomorrow.year()
        ))
      } else {
        Ok(format!(
          "{} {} {} {} * {}",
          m,
          h,
          now.day(),
          now.month(),
          now.year()
        ))
      }
    }
    d => Ok(format!("{} {} * * {} *", m, h, d)),
  }
}

pub fn parse_alarm_args<'a, Z>(input: &'a str, tz: &Z) -> Result<CronArgs<'a>, &'static str>
where
  Z: TimeZone,
  Z::Offset: Display,
{
  let first_hash = input.find('#');
  let title = match first_hash {
    Some(first_hash) => &input[first_hash..],
    None => "",
  };
  let mut alarm_str = String::from(match first_hash {
    Some(first_hash) => &input[..first_hash],
    None => input,
  });
  let time_str = test_time_str(alarm_str.as_str(), tz);
  if let Ok(time_str) = time_str {
    alarm_str = String::from(time_str)
  }
  let cron_str = test_cron(alarm_str.as_str())?;
  Ok(CronArgs {
    cron: cron_str,
    title,
  })
}

pub fn with_alarm_id<T>(store: &Store, user_id: i64, cmd: &Command, f: T) -> InputMessageContent
where
  T: Fn(&mut Vec<Alarm>, usize) -> InputMessageContent,
{
  let id = cmd.arg().parse::<usize>();
  if let Err(_) = id {
    return build_fmt_message(|f| f_bad_arguments(f, "闹钟编号格式有误。"));
  }
  let id = id.unwrap();
  let to_send = {
    let state = store.state();
    let alarms = state.alarms.borrow();
    let user_alarms = alarms.get(&user_id);
    match user_alarms {
      None => build_fmt_message(|f| f_bad_arguments(f, "没有这个编号的闹钟。")),
      Some(alarms) => {
        let mut alarms = alarms.borrow_mut();
        if id >= alarms.len() {
          build_fmt_message(|f| f_bad_arguments(f, "没有这个编号的闹钟。"))
        } else {
          f(&mut alarms, id)
        }
      }
    }
  };
  store.save().expect("Failed to save state");
  return to_send;
}
