use hyper_bed_caller::handler::*;

fn main() {
  let (tdlib, store) = initialize_app("/data/store.json");
  let handler = start_handler(tdlib.clone(), store.clone());
  let cron = start_cron(tdlib, store);
  handler.join().expect("Handler thread failed");
  cron.join().expect("Cron thread failed");
}
