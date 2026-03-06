use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = env!("CARGO_PKG_NAME"), max_term_width = 80)]
#[command(about = "rpi-zero2w-strava-dash")]
#[command(version)]
pub struct Args {
    /// Force a Strava auth flow, to get a token that has read scope
    /// to all activities
    #[arg(short, long)]
    pub auth: bool,

    /// Run a single cycle (fetch → render → display) and exit
    #[arg(long)]
    pub once: bool,

    /// Save the rendered dashboard as a PNG file (for testing without e-paper)
    #[arg(long, value_name = "PATH")]
    pub save_png: Option<String>,
}
