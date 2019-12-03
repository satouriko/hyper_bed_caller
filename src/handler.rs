use std::{cell::RefCell, env, io, sync::Arc, thread, time};
extern crate uname;
use crate::{alarm::*, cmd::*, cron::*, fmt::*, store::*};
use chrono;
use chrono_tz::Tz;
use rtdlib::{tdjson::Tdlib, types::*};

pub fn initialize_app<T>(path: T) -> (Arc<Tdlib>, Arc<Store>)
where
  T: AsRef<str>,
{
  let tdlib = Arc::new(Tdlib::new());
  let set_online = SetOption::builder()
    .name("online")
    .value(OptionValue::Boolean(
      OptionValueBoolean::builder().value(true).build(),
    ))
    .build();
  tdlib.send(&set_online.to_json().expect("Bad JSON"));
  let store = Arc::new(Store::new(path));
  return (tdlib, store);
}

pub fn start_handler(tdlib: Arc<Tdlib>, store: Arc<Store>) -> thread::JoinHandle<()> {
  let mut user_name = String::default();
  let phone_number = env::var("PHONE").expect("Unknown env phone number");
  let phone_number = if phone_number.starts_with("+") {
    phone_number[1..].to_string()
  } else {
    phone_number
  };
  thread::spawn(move || loop {
    let json = tdlib.receive(60.0);
    if let None = json {
      continue;
    }
    let json = json.unwrap();
    let td_type = detect_td_type(json.as_str());
    if let None = td_type {
      eprintln!("data failed with json");
      continue;
    };
    let td_type = td_type.unwrap();
    match td_type.as_str() {
      "updateAuthorizationState" => {
        let state: UpdateAuthorizationState =
          serde_json::from_str(json.as_str()).unwrap_or_default();
        let req: Box<dyn RObject> = match state.authorization_state() {
          AuthorizationState::WaitTdlibParameters(_) => Box::new(
            SetTdlibParameters::builder()
              .parameters(
                TdlibParameters::builder()
                  .database_directory("tdlib")
                  .use_message_database(true)
                  .use_secret_chats(true)
                  .api_id(env!("API_ID").parse::<i64>().expect("Bad API ID"))
                  .api_hash(env!("API_HASH"))
                  .system_language_code("en")
                  .device_model("Desktop")
                  .system_version(uname::uname().expect("Bad uname").sysname)
                  .application_version(env!("CARGO_PKG_VERSION"))
                  .enable_storage_optimizer(true)
                  .build(),
              )
              .build(),
          ),
          AuthorizationState::WaitEncryptionKey(_) => {
            Box::new(SetDatabaseEncryptionKey::builder().build())
          }
          AuthorizationState::WaitPhoneNumber(_) => Box::new(
            SetAuthenticationPhoneNumber::builder()
              .phone_number(&phone_number)
              .build(),
          ),
          AuthorizationState::WaitCode(code) => {
            let prompt = match code.code_info().type_() {
              AuthenticationCodeType::TelegramMessage(_) => {
                String::from("Telegram has sent the code to the Telegram app on your other device.")
              }
              AuthenticationCodeType::Sms(_) => format!(
                "Telegram has sent an SMS with an activation code to your phone {}.",
                code.code_info().phone_number()
              ),
              _ => String::from("Telegram is calling you."),
            };
            println!("{}", prompt);
            println!("Please type authentication code:");
            let mut input = String::new();
            io::stdin().read_line(&mut input).expect("Bad input");
            Box::new(CheckAuthenticationCode::builder().code(input).build())
          }
          AuthorizationState::Ready(_) => Box::new(GetMe::builder().build()),
          _ => {
            println!("{}\t{}", td_type, json);
            panic!("Unsupported authorization");
          }
        };
        tdlib.send(&req.to_json().expect("Bad JSON"));
      }
      "user" => {
        let user: User = serde_json::from_str(json.as_str()).unwrap_or_default();
        if user.phone_number().as_str() == phone_number.as_str() {
          user_name = user.username().to_string();
        }
      }
      "updateNewMessage" => {
        let update_new_message: UpdateNewMessage =
          serde_json::from_str(json.as_str()).unwrap_or_default();
        let message = update_new_message.message();
        if message.is_outgoing() {
          continue;
        }
        let reply_text_msg = |msg: InputMessageContent| {
          let req = SendMessage::builder()
            .chat_id(message.chat_id())
            .input_message_content(msg)
            .reply_to_message_id(message.id())
            .build();
          tdlib.send(&req.to_json().expect("Bad JSON"));
        };
        match message.content() {
          MessageContent::MessageText(message_text) => {
            let text = message_text.text().text();
            if text == "/help" || text == &format!("/help@{}", user_name) {
              reply_text_msg(build_fmt_message(f_help_message));
              continue;
            }
            if !text.starts_with("#") {
              let mut toggled = false;
              {
                let state = store.state();
                let user_alarms = state.alarms.get(&message.sender_user_id());
                if let None = user_alarms {
                  continue;
                }
                let user_alarms = user_alarms.unwrap();
                let mut alarms = user_alarms.borrow_mut();
                for alarm in alarms.iter_mut() {
                  if alarm.is_strict && alarm.is_informing {
                    if text == alarm.strict_challenge.as_str() {
                      alarm.is_informing = false;
                      toggled = true;
                      reply_text_msg(if alarm.title == "" {
                        build_plain_message(format!("闹钟已关闭。"))
                      } else {
                        build_plain_message(format!("闹钟 {} 已关闭。", alarm.title))
                      });
                      break;
                    }
                  }
                }
              }
              if toggled {
                store.save().expect("Failed to save state");
              }
              continue;
            }
            let text = message_text.text().text();
            let cmd = parse_command_msg(text);
            let handle_alarm = |is_strict: bool| {
              let tz = {
                let state = store.state();
                let tz = state.timezone.get(&message.sender_user_id());
                match tz {
                  Some(tz) => {
                    let tz = tz.parse::<Tz>().unwrap();
                    Some(tz)
                  }
                  None => None,
                }
              };
              let alarm_args = {
                match tz {
                  Some(tz) => parse_alarm_args(cmd.arg(), &tz),
                  None => parse_alarm_args(cmd.arg(), &chrono::Local),
                }
              };
              let to_send = match alarm_args {
                Err(error) => Err(error),
                Ok(cron_args) => {
                  let alarm = Alarm::new(
                    message.sender_user_id(),
                    message.chat_id(),
                    cron_args.cron(),
                    cron_args.title(),
                    is_strict,
                  );
                  let mut state = store.state();
                  let user_alarms = state.alarms.get(&message.sender_user_id());
                  if let None = user_alarms {
                    state
                      .alarms
                      .insert(message.sender_user_id(), RefCell::new(vec![]));
                  }
                  let mut user_alarms = state
                    .alarms
                    .get(&message.sender_user_id())
                    .unwrap()
                    .borrow_mut();
                  user_alarms.push(alarm);
                  let next_alarm = match tz {
                    Some(tz) => get_next_schedule(cron_args.cron(), tz).to_string(),
                    None => get_next_schedule(cron_args.cron(), chrono::Local.clone()).to_string(),
                  };
                  let next_alarm = match next_alarm {
                    Some(next_alarm) => format!("下次闹钟时间：{}", next_alarm),
                    None => format!("但是它看起来并不会响。"),
                  };
                  Ok(match cron_args.title() {
                    "" => format!("闹钟已设置。{}", next_alarm),
                    _ => format!("闹钟 {} 已设置。{}", cron_args.title(), next_alarm),
                  })
                }
              };
              match to_send {
                Ok(to_send) => {
                  store.save().expect("Failed to save state");
                  reply_text_msg(build_plain_message(to_send));
                }
                Err(_) => {
                  reply_text_msg(build_fmt_message(|f| f_bad_arguments(f, "无效的表达式。")));
                }
              }
            };
            match cmd.cmd() {
              "#help" => {
                reply_text_msg(build_fmt_message(f_help_message));
              }
              "#timezone" => {
                if cmd.arg() == "" {
                  let state = store.state();
                  let tz = state.timezone.get(&message.sender_user_id());
                  let current_tz_str = match tz {
                    Some(tz) => tz.clone(),
                    None => chrono::Local::now().format("%Z").to_string(),
                  };
                  reply_text_msg(build_plain_message(format!("当前时区：{}", current_tz_str)));
                  continue;
                }
                let tz = cmd.arg().parse::<Tz>();
                let to_send = match tz {
                  Err(_) => build_fmt_message(|f| f_bad_arguments(f, "没有这个时区。")),
                  Ok(_) => {
                    let mut state = store.state();
                    state
                      .timezone
                      .insert(message.sender_user_id(), String::from(cmd.arg()));
                    build_plain_message(format!("时区已更新为 {}。", cmd.arg()))
                  }
                };
                store.save().expect("Failed to save state");
                reply_text_msg(to_send);
              }
              "#alarm" => {
                handle_alarm(false);
              }
              "#alarm!" => {
                handle_alarm(true);
              }
              "#list" => {
                let state = store.state();
                let tz = state.timezone.get(&message.sender_user_id());
                let tz = match tz {
                  Some(tz) => {
                    let tz = tz.parse::<Tz>().unwrap();
                    Some(tz)
                  }
                  None => None,
                };
                let user_alarms = state.alarms.get(&message.sender_user_id());
                let to_send = match user_alarms {
                  None => build_plain_message("还一个闹钟都没有呢。"),
                  Some(alarms) => build_fmt_message(|f| match tz {
                    Some(tz) => f_list_alarms(f, &alarms.borrow(), tz),
                    None => f_list_alarms(f, &alarms.borrow(), chrono::Local.clone()),
                  }),
                };
                reply_text_msg(to_send);
              }
              "#disalarm" => {
                if cmd.arg() == "" {
                  let to_send = {
                    let now = chrono::Local::now().timestamp();
                    let state = store.state();
                    let user_alarms = state.alarms.get(&message.sender_user_id());
                    match user_alarms {
                      None => build_fmt_message(|f| {
                        f_bad_arguments(f, "没有要响的闹钟了，去设置一些吧。")
                      }),
                      Some(alarms) => {
                        let mut alarms = alarms.borrow_mut();
                        let tz = state.timezone.get(&message.sender_user_id());
                        let disalarm_if_in_an_hour =
                          |t: i64,
                           s: Option<String>,
                           a: Option<&mut Alarm>|
                           -> InputMessageContent {
                            if let None = a {
                              return build_fmt_message(|f| {
                                f_bad_arguments(f, "没有要响的闹钟了，去设置一些吧。")
                              });
                            }
                            let a = a.unwrap();
                            let s = s.unwrap();
                            if a.is_pending {
                              return build_plain_message(
                                "你不能移除正在响铃的闹钟，请先关闭闹钟。",
                              );
                            }
                            if a.is_informing && a.is_strict {
                              return build_plain_message(
                                "你不能移除正在进行的闹钟，请先关闭闹钟。",
                              );
                            }
                            if a.is_informing {
                              a.is_informing = false;
                              return if a.title == "" {
                                build_plain_message(format!("已关闭正在进行的闹钟。"))
                              } else {
                                build_plain_message(format!("已关闭正在进行的闹钟 {}。", a.title))
                              };
                            }
                            if t >= now && t < now + 3600 {
                              a.is_oneoff = true;
                              return if a.title == "" {
                                build_plain_message(format!("已取消预定于 {} 的闹钟。", s))
                              } else {
                                build_plain_message(format!(
                                  "已取消预定于 {} 的闹钟 {}。",
                                  s, a.title
                                ))
                              };
                            }
                            return build_fmt_message(|f| {
                              f_bad_arguments(f, "最近没有要响的闹钟。")
                            });
                          };
                        match tz {
                          Some(tz) => {
                            let mut next_alarm =
                              get_recent_schedule_mut(&mut *alarms, tz.parse::<Tz>().unwrap());
                            disalarm_if_in_an_hour(
                              next_alarm.schedule().to_timestamp(),
                              next_alarm.schedule().to_string(),
                              next_alarm.alarm_mut(),
                            )
                          }
                          None => {
                            let mut next_alarm =
                              get_recent_schedule_mut(&mut *alarms, chrono::Local.clone());
                            disalarm_if_in_an_hour(
                              next_alarm.schedule().to_timestamp(),
                              next_alarm.schedule().to_string(),
                              next_alarm.alarm_mut(),
                            )
                          }
                        }
                      }
                    }
                  };
                  store.save().expect("Failed to save state");
                  reply_text_msg(to_send);
                  continue;
                }
                let id = cmd.arg().parse::<usize>();
                if let Err(_) = id {
                  reply_text_msg(build_fmt_message(|f| {
                    f_bad_arguments(f, "闹钟编号格式有误。")
                  }));
                  continue;
                }
                let id = id.unwrap();
                let to_send = {
                  let state = store.state();
                  let user_alarms = state.alarms.get(&message.sender_user_id());
                  match user_alarms {
                    None => build_fmt_message(|f| f_bad_arguments(f, "没有这个编号的闹钟。")),
                    Some(alarms) => {
                      let mut alarms = alarms.borrow_mut();
                      if id >= alarms.len() {
                        build_fmt_message(|f| f_bad_arguments(f, "没有这个编号的闹钟。"))
                      } else {
                        if alarms[id].is_strict && alarms[id].is_informing {
                          build_plain_message("你不能移除正在进行的闹钟，请先关闭闹钟。")
                        } else {
                          alarms.remove(id);
                          build_plain_message("闹钟已移除。")
                        }
                      }
                    }
                  }
                };
                store.save().expect("Failed to save state");
                reply_text_msg(to_send);
              }
              "#strict" => {
                let id = cmd.arg().parse::<usize>();
                if let Err(_) = id {
                  reply_text_msg(build_fmt_message(|f| {
                    f_bad_arguments(f, "闹钟编号格式有误。")
                  }));
                  continue;
                }
                let id = id.unwrap();
                let to_send = {
                  let state = store.state();
                  let user_alarms = state.alarms.get(&message.sender_user_id());
                  match user_alarms {
                    None => build_fmt_message(|f| f_bad_arguments(f, "没有这个编号的闹钟。")),
                    Some(alarms) => {
                      let mut alarms = alarms.borrow_mut();
                      if id >= alarms.len() {
                        build_fmt_message(|f| f_bad_arguments(f, "没有这个编号的闹钟。"))
                      } else {
                        if alarms[id].is_informing {
                          build_plain_message("你不能对正在进行的闹钟使用此命令。")
                        } else {
                          alarms[id].is_strict = !alarms[id].is_strict;
                          let alarm_text = match alarms[id].title.as_str() {
                            "" => format!("[{}]", id),
                            title => format!("[{}] {}", id, title),
                          };
                          build_plain_message(match alarms[id].is_strict {
                            true => format!("已变更闹钟 {} 为严格模式。", alarm_text),
                            false => format!("已取消闹钟 {} 的严格模式。", alarm_text),
                          })
                        }
                      }
                    }
                  }
                };
                store.save().expect("Failed to save state");
                reply_text_msg(to_send);
              }
              "#next" => {
                let state = store.state();
                let user_alarms = state.alarms.get(&message.sender_user_id());
                if let None = user_alarms {
                  continue;
                }
                let alarms = user_alarms.unwrap().borrow();
                let tz = state.timezone.get(&message.sender_user_id());
                let (time_str, alarm_title) = match tz {
                  Some(tz) => {
                    let next_alarm = get_recent_schedule(&alarms, tz.parse::<Tz>().unwrap());
                    (next_alarm.schedule().to_string(), next_alarm.alarm_title())
                  }
                  None => {
                    let next_alarm = get_recent_schedule(&alarms, chrono::Local.clone());
                    (next_alarm.schedule().to_string(), next_alarm.alarm_title())
                  }
                };
                let to_send = match time_str {
                  Some(time_str) => format!("下次闹钟时间：{} {}", time_str, alarm_title),
                  None => format!("没有要响的闹钟了。"),
                };
                reply_text_msg(build_plain_message(to_send));
              }
              "#purge" => {
                let purged_cnt = {
                  let state = store.state();
                  let user_alarms = state.alarms.get(&message.sender_user_id());
                  if let None = user_alarms {
                    reply_text_msg(build_plain_message("还一个闹钟都没有呢。"));
                    continue;
                  }
                  let mut alarms = user_alarms.unwrap().borrow_mut();
                  let tz = state.timezone.get(&message.sender_user_id());
                  let tz = match tz {
                    Some(tz) => {
                      let tz = tz.parse::<Tz>().unwrap();
                      Some(tz)
                    }
                    None => None,
                  };
                  // awaiting https://doc.rust-lang.org/std/vec/struct.Vec.html#method.drain_filter
                  let mut i = 0;
                  let mut purged_cnt = 0;
                  while i != alarms.len() {
                    if alarms[i].is_informing {
                      i += 1;
                      continue;
                    }
                    match tz {
                      Some(tz) => {
                        let next_alarm = get_next_schedule(&alarms[i].cron, tz);
                        if !next_alarm.has_schedule() {
                          alarms.remove(i);
                          purged_cnt += 1;
                        } else {
                          i += 1;
                        }
                      }
                      None => {
                        let next_alarm = get_next_schedule(&alarms[i].cron, chrono::Local.clone());
                        if !next_alarm.has_schedule() {
                          alarms.remove(i);
                          purged_cnt += 1;
                        } else {
                          i += 1;
                        }
                      }
                    }
                  }
                  purged_cnt
                };
                if purged_cnt > 0 {
                  store.save().expect("Failed to save state");
                  reply_text_msg(build_plain_message(format!(
                    "已清除 {} 个闹钟。",
                    purged_cnt
                  )));
                } else {
                  reply_text_msg(build_plain_message("没有过期的闹钟。"));
                }
              }
              _ => {
                continue;
              }
            }
          }
          _ => (),
        }
      }
      "updateCall" => {
        let update_call: UpdateCall = serde_json::from_str(json.as_str()).unwrap_or_default();
        let call = update_call.call();
        let user_id = call.user_id();
        if !call.is_outgoing() {
          if let CallState::Pending(_) = call.state() {
            let req = DiscardCall::builder().call_id(call.id()).build();
            tdlib.send(&req.to_json().expect("Bad JSON"));
          }
          continue;
        }
        match call.state() {
          CallState::ExchangingKeys(_) => {
            let state = store.state();
            let user_alarms = state.alarms.get(&user_id);
            if let None = user_alarms {
              continue;
            }
            let user_alarms = user_alarms.unwrap();
            let mut alarms = user_alarms.borrow_mut();
            for alarm in alarms.iter_mut() {
              if alarm.is_pending {
                alarm.is_pending = false;
                if !alarm.is_strict {
                  alarm.is_informing = false;
                } else {
                  let (challenge, answer, map) = generate_strict_challenge();
                  alarm.strict_challenge = answer;
                  let req = SendMessage::builder()
                    .chat_id(user_id)
                    .input_message_content(build_fmt_message(|f| {
                      f_strict_challenge(f, &challenge, &map)
                    }))
                    .build();
                  tdlib.send(&req.to_json().expect("Bad JSON"));
                }
              }
            }
          }
          CallState::Discarded(_) => {
            let state = store.state();
            let user_alarms = state.alarms.get(&user_id);
            if let None = user_alarms {
              continue;
            }
            let user_alarms = user_alarms.unwrap();
            let mut alarms = user_alarms.borrow_mut();
            for alarm in alarms.iter_mut() {
              if alarm.is_pending {
                alarm.is_pending = false;
              }
            }
          }
          _ => {}
        }
        store.save().expect("Failed to save state")
      }
      _ => {
        println!("{}\t{}", td_type, json);
      }
    };
  })
}

pub fn start_cron(tdlib: Arc<Tdlib>, store: Arc<Store>) -> thread::JoinHandle<()> {
  let mut service = CronService::new();
  thread::spawn(move || loop {
    thread::sleep(time::Duration::from_secs(1));
    service.tick(|lask_tick, now| {
      {
        let state = store.state();
        let alarms_map = &state.alarms;
        for (user_id, user_alarms) in alarms_map {
          let tz = state.timezone.get(user_id);
          let tz = match tz {
            Some(tz) => {
              let tz = tz.parse::<Tz>().unwrap();
              Some(tz)
            }
            None => None,
          };
          let mut alarms = user_alarms.borrow_mut();
          for alarm in alarms.iter_mut() {
            let should_alarm = match tz {
              Some(tz) => {
                let next_alarm = match alarm.is_informing {
                  false => get_next_schedule(&alarm.cron, tz).to_timestamp(),
                  true => alarm.reschedule,
                };
                next_alarm > lask_tick && next_alarm <= now
              }
              None => {
                let next_alarm = match alarm.is_informing {
                  false => get_next_schedule(&alarm.cron, chrono::Local.clone()).to_timestamp(),
                  true => alarm.reschedule,
                };
                next_alarm > lask_tick && next_alarm <= now
              }
            };
            if should_alarm {
              if alarm.is_pending {
                continue;
              }
              // is_oneoff * is_informing = 0
              if alarm.is_oneoff {
                alarm.is_oneoff = false;
                continue;
              }
              alarm.is_pending = true;
              alarm.is_informing = true;
              alarm.reschedule = now + 300;
              if alarm.title != "" {
                let req = SendMessage::builder()
                  .chat_id(*user_id)
                  .input_message_content(build_plain_message(&alarm.title))
                  .build();
                tdlib.send(&req.to_json().expect("Bad JSON"));
              }
              let req = CreateCall::builder()
                .user_id(*user_id)
                .protocol(
                  CallProtocol::builder()
                    .udp_p2p(true)
                    .udp_reflector(true)
                    .min_layer(65)
                    .max_layer(65),
                )
                .build();
              tdlib.send(&req.to_json().expect("Bad JSON"));
            }
          }
        }
      }
      store.save().expect("Failed to save state");
    });
  })
}
