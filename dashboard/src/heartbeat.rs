use std::io::Write;
use std::path::PathBuf;

fn heartbeat_path() -> PathBuf {
  dirs::cache_dir().unwrap_or_else(|| PathBuf::from(".cache"))
                   .join(env!("CARGO_PKG_NAME"))
                   .join("heartbeat")
}

pub fn write_heartbeat(message: &str) {
  let path = heartbeat_path();
  if let Some(parent) = path.parent() {
    let _ = std::fs::create_dir_all(parent);
  }

  let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
  let file = std::fs::OpenOptions::new().create(true).append(true).open(path);

  match file {
    Ok(mut f) => {
      let _ = writeln!(f, "{now}: {message}");
    },
    Err(e) => {
      log::warn!("Failed to write heartbeat: {e}");
    },
  }
}
