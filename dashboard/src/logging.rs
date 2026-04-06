use std::io::IsTerminal;

/// Sets up the logging for our app
/// Remove timestamps if not running in terminal (syslog has its own timestamp)
pub fn setup() {
  let mut log_builder =
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));

  if std::io::stderr().is_terminal() {
    // Compact timestamp + colored level for interactive use
    log_builder.format(|buf, record| {
                 use std::io::Write;
                 let now = chrono::Local::now();
                 let level = record.level();
                 let style = buf.default_level_style(level);
                 writeln!(buf,
                          "{} [{style}{}{style:#} {}] {}",
                          now.format("%Y/%m/%d %H:%M"),
                          level,
                          record.module_path().unwrap_or(""),
                          record.args())
               });
  } else {
    // No timestamp under systemd (journalctl provides its own)
    log_builder.format(|buf, record| {
                 use std::io::Write;
                 writeln!(buf,
                          "[{} {}] {}",
                          record.level(),
                          record.module_path().unwrap_or(""),
                          record.args())
               });
  }
  log_builder.init();
}
