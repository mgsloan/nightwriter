use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    about = "Grabs keyboard and behaves like an append-only text editor until you press ctrl+alt+escape."
)]
pub struct Opt {
    #[structopt(long = "debug")]
    pub debug: bool,
    #[structopt(parse(from_os_str), help = "File to write text to")]
    pub output_file: Option<PathBuf>,
    #[structopt(parse(from_os_str), help = "Override configuration file location")]
    pub config: Option<PathBuf>,
}
