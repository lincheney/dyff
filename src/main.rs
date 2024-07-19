use clap::Parser;

#[derive(Clone, Debug, clap::ValueEnum)]
enum ColorChoices {
    Never,
    Auto,
    Always,
}

#[derive(Debug, clap::Parser)]
#[command(name = "diff")]
struct Cli {

    #[arg(long, value_enum, default_value_t = ColorChoices::Always)]
    color: ColorChoices,

    #[arg(short = 'N', long = "no-line-numbers", action = clap::ArgAction::SetFalse)]
    line_numbers: bool,

    #[arg(short, long)]
    signs: bool,

    #[arg(long)]
    exact: bool,

    #[arg(short, long)]
    filter: Option<String>,

    /// use LABEL instead of file name and timestamp (can be repeated)
    #[arg(long)]
    label: Vec<String>,

    file1: Option<String>,
    file2: Option<String>,

    #[arg(allow_hyphen_values = true)]
    extras: Vec<String>,
}

fn main() {
    let cli = Cli::parse();
    eprintln!("DEBUG(marry) \t{}\t= {:?}", stringify!(cli), cli);
}
