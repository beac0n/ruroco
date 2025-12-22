use chrono::Utc;

pub(crate) fn info(msg: &str) {
    let date_time = get_date_time();
    println!("[{date_time} \x1b[32mINFO\x1b[0m ] {msg}")
}

pub(crate) fn error(msg: impl std::fmt::Display) {
    let date_time = get_date_time();
    eprintln!("[{date_time} \x1b[31mERROR\x1b[0m ] {msg}")
}

fn get_date_time() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}
