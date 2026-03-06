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
}
