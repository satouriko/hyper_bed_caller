use std::{cell::RefCell, env, io, sync::Arc, thread, time};
extern crate uname;
use crate::{cmd::*, cron::*, fmt::*, store::*};
use chrono;
use chrono_tz::Tz;
use rtdlib::{tdjson::Tdlib, types::*};

pub fn initialize_app(path: &str) -> (Arc<Tdlib>, Arc<Store>) {
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
                let req: Option<Box<dyn RObject>> = match state.authorization_state() {
                    AuthorizationState::WaitTdlibParameters(_) => Some(Box::new(
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
                    )),
                    AuthorizationState::WaitEncryptionKey(_) => {
                        Some(Box::new(SetDatabaseEncryptionKey::builder().build()))
                    }
                    AuthorizationState::WaitPhoneNumber(_) => Some(Box::new(
                        SetAuthenticationPhoneNumber::builder()
                            .phone_number(env::var("PHONE").expect("Unknown env phone number"))
                            .build(),
                    )),
                    AuthorizationState::WaitCode(code) => {
                        let prompt = match code.code_info().type_() {
                            AuthenticationCodeType::TelegramMessage(_) => {
                                String::from(
                                    "Telegram has sent the code to the Telegram app on your other device."
                                )
                            }
                            AuthenticationCodeType::Sms(_) => {
                                format!(
                                    "Telegram has sent an SMS with an activation code to your phone {}.",
                                     code.code_info().phone_number()
                                )
                            }
                            _ => {
                                String::from("Telegram is calling you.")
                            }
                        };
                        println!("{}", prompt);
                        println!("Please type authentication code:");
                        let mut input = String::new();
                        io::stdin().read_line(&mut input).expect("Bad input");
                        Some(Box::new(
                            CheckAuthenticationCode::builder().code(input).build(),
                        ))
                    }
                    _ => {
                        println!("{}\t{}", td_type, json);
                        None
                    }
                };
                if let Some(req) = req {
                    tdlib.send(&req.to_json().expect("Bad JSON"))
                }
            }
            "updateNewMessage" => {
                let update_new_message: UpdateNewMessage =
                    serde_json::from_str(json.as_str()).unwrap_or_default();
                let message = update_new_message.message();
                if message.is_outgoing() {
                    continue;
                }
                println!("{}", json);
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
                        if text == "/help"
                            || text
                                == &format!(
                                    "/help@{}",
                                    env::var("BOT_USERNAME").unwrap_or_default()
                                )
                        {
                            reply_text_msg(build_fmt_message(f_help_message));
                            continue;
                        }
                        if !text.starts_with("#") {
                            continue;
                        }
                        let text = message_text.text().text();
                        let cmd = parse_command_msg(text);
                        match cmd.cmd() {
                            "#help" => {
                                reply_text_msg(build_fmt_message(f_help_message));
                            }
                            "#timezone" => {
                                if cmd.arg() == "" {
                                    let state = store.state();
                                    let tz = state.timezones.get(&message.sender_user_id());
                                    let current_tz_str = match tz {
                                        Some(tz) => tz.clone(),
                                        None => chrono::Local::now().format("%Z").to_string(),
                                    };
                                    reply_text_msg(build_plain_message(format!(
                                        "当前时区：{}",
                                        current_tz_str
                                    )));
                                    continue;
                                }
                                let tz = cmd.arg().parse::<Tz>();
                                let to_send = match tz {
                                    Err(_) => String::from("没有这个时区。"),
                                    Ok(_) => {
                                        let mut state = store.state();
                                        state.timezones.insert(
                                            message.sender_user_id(),
                                            String::from(cmd.arg()),
                                        );
                                        format!("时区已更新为 {}。", cmd.arg())
                                    }
                                };
                                store.save().expect("Failed to save state");
                                reply_text_msg(build_plain_message(to_send));
                            }
                            "#alarm" => {
                                let tz = {
                                    let state = store.state();
                                    let tz = state.timezones.get(&message.sender_user_id());
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
                                        let alarm = Alarm {
                                            user_id: message.sender_user_id(),
                                            chat_id: message.chat_id(),
                                            cron: String::from(cron_args.cron()),
                                            title: String::from(cron_args.title()),
                                            is_strict: false,
                                        };
                                        let mut state = store.state();
                                        let user_alarms =
                                            state.alarms.get(&message.sender_user_id());
                                        if let None = user_alarms {
                                            state.alarms.insert(
                                                message.sender_user_id(),
                                                RefCell::new(vec![]),
                                            );
                                        }
                                        let mut user_alarms = state
                                            .alarms
                                            .get(&message.sender_user_id())
                                            .unwrap()
                                            .borrow_mut();
                                        user_alarms.push(alarm);
                                        let next_alarm = match tz {
                                            Some(tz) => {
                                                get_next_schedule(cron_args.cron(), tz).to_string()
                                            }
                                            None => get_next_schedule(
                                                cron_args.cron(),
                                                chrono::Local.clone(),
                                            )
                                            .to_string(),
                                        };
                                        let next_alarm = match next_alarm {
                                            Some(next_alarm) => {
                                                format!("下次闹钟时间：{}", next_alarm)
                                            }
                                            None => format!("但是它看起来并不会响。"),
                                        };
                                        Ok(match cron_args.title() {
                                            "" => format!("闹钟已设置。{}", next_alarm),
                                            _ => format!(
                                                "闹钟 {} 已设置。{}",
                                                cron_args.title(),
                                                next_alarm
                                            ),
                                        })
                                    }
                                };
                                match to_send {
                                    Ok(to_send) => {
                                        store.save().expect("Failed to save state");
                                        reply_text_msg(build_plain_message(to_send));
                                    }
                                    Err(_) => {
                                        reply_text_msg(build_fmt_message(f_bad_cron_expression));
                                    }
                                }
                            }
                            "#list" => {
                                let state = store.state();
                                let user_alarms = state.alarms.get(&message.sender_user_id());
                                let to_send = match user_alarms {
                                    None => String::from("还一个闹钟都没有呢。"),
                                    Some(alarms) => {
                                        let mut to_send = String::from("");
                                        for (i, alarm) in alarms.borrow().iter().enumerate() {
                                            to_send += &format!(
                                                "[{}] {} {} {}\n",
                                                i, alarm.cron, alarm.title, alarm.is_strict
                                            );
                                        }
                                        to_send
                                    }
                                };
                                reply_text_msg(build_plain_message(to_send));
                            }
                            "#disalarm" => {
                                let id = cmd.arg().parse::<usize>();
                                if let Err(_) = id {
                                    reply_text_msg(build_plain_message("闹钟编号格式有误。"));
                                    continue;
                                }
                                let id = id.unwrap();
                                let to_send = {
                                    let state = store.state();
                                    let user_alarms = state.alarms.get(&message.sender_user_id());
                                    match user_alarms {
                                        None => String::from("没有这个编号的闹钟。"),
                                        Some(alarms) => {
                                            let mut alarms = alarms.borrow_mut();
                                            if id >= alarms.len() {
                                                String::from("没有这个编号的闹钟。")
                                            } else {
                                                alarms.remove(id);
                                                String::from("闹钟已移除。")
                                            }
                                        }
                                    }
                                };
                                store.save().expect("Failed to save state");
                                reply_text_msg(build_plain_message(to_send));
                            }
                            "#next" => {
                                let state = store.state();
                                let user_alarms = state.alarms.get(&message.sender_user_id());
                                if let None = user_alarms {
                                    continue;
                                }
                                let alarms = user_alarms.unwrap().borrow();
                                let tz = state.timezones.get(&message.sender_user_id());
                                let (tz_str, alarm_title) = match tz {
                                    Some(tz) => {
                                        let next_alarm =
                                            get_recent_schedule(&alarms, tz.parse::<Tz>().unwrap());
                                        (
                                            next_alarm.schedule().to_string(),
                                            next_alarm.alarm_title(),
                                        )
                                    }
                                    None => {
                                        let next_alarm =
                                            get_recent_schedule(&alarms, chrono::Local.clone());
                                        (
                                            next_alarm.schedule().to_string(),
                                            next_alarm.alarm_title(),
                                        )
                                    }
                                };
                                let to_send = match tz_str {
                                    Some(tz_str) => {
                                        format!("下次闹钟时间：{} {}", tz_str, alarm_title)
                                    }
                                    None => format!("没有要响的闹钟了。"),
                                };
                                reply_text_msg(build_plain_message(to_send));
                            }
                            _ => {
                                continue;
                            }
                        }
                    }
                    _ => (),
                }
            }
            _ => {
                println!("{}\t{}", td_type, json);
            }
        };
    })
}

pub fn start_cron(tdlib: Arc<Tdlib>, store: Arc<Store>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        thread::sleep(time::Duration::from_secs(5));
        let get_me = GetMe::builder().build();
        tdlib.send(&get_me.to_json().expect("Bad JSON"));
        println!("{:?}", store.state());
    })
}
