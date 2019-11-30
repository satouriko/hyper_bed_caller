use hyper_bed_caller::handler::*;

fn main() {
    let tdlib = initialize_app();
    let handler = start_handler(tdlib.clone());
    let cron = start_cron(tdlib);
    handler.join().expect("Handler thread failed");
    cron.join().expect("Cron thread failed");
}
