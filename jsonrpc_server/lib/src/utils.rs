use env_logger::Env;
use log::debug;
use std::io::Write;

pub(crate) fn debug_limited(msg: &String, max_len: usize) {
    let truncated = if msg.len() > max_len {
        &msg[..max_len]
    } else {
        &msg
    };
    debug!("{}", truncated);
}

pub fn setup_logging() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn"))
    .format(|buf, record| {
        let file_name = record.file().unwrap_or("unknown");
        let file_name = std::path::Path::new(file_name)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();

        writeln!(
            buf,
            "{} [{}] {}:{} - {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            record.level(),
            file_name,
            record.line().unwrap_or(0),
            record.args()
        )
    })
    .init();
}