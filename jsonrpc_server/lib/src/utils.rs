use env_logger::Env;
use log::debug;
use std::io::Write;
use ansi_term::Colour;

pub(crate) fn debug_limited(msg: &String, max_len: usize) {
    let truncated = if msg.len() > max_len {
        &msg[..max_len]
    } else {
        msg
    };
    debug!("{}", truncated);
}

pub fn setup_logging() {
    let start_time = std::time::Instant::now();
    let _ = env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format(move |buf, record| {
            let file_name = record.file().unwrap_or("unknown");
            let file_name = std::path::Path::new(file_name)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();

            // Calculate elapsed time since start in seconds with 3 decimal places
            let elapsed = start_time.elapsed();
            let timestamp = format!("{:.3}", elapsed.as_secs_f32());

            // Color the level based on its severity
            let level = match record.level() {
                log::Level::Error => Colour::Red.paint("ERROR"),
                log::Level::Warn => Colour::Yellow.paint("WARN "),
                log::Level::Info => Colour::Green.paint("INFO "),
                log::Level::Debug => Colour::Fixed(8).paint("DEBUG"),
                log::Level::Trace => Colour::Purple.paint("TRACE"),
            };

            writeln!(
                buf,
                "{} {} {} {}",
                Colour::Fixed(8).paint(timestamp),
                level,
                Colour::Fixed(8).paint(format!("{}:{}", file_name, record.line().unwrap_or(0))),
                record.args()
            )
        })
        .try_init();
}