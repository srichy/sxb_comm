use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[arg(short, long, value_enum)]
    action: Action,

    #[arg(help="Filename or address depending on action")]
    argument: Option<String>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Action {
    HexDump,
    ReadBinary,
    WriteBinary,
    Execute,
    DumpRegisters,
}

fn main() {
    let cli = Args::parse();

    match cli.action {

        Action::HexDump => {
        }

        Action::ReadBinary => {
        }

        Action::WriteBinary => {
        }

        Action::Execute => {
        }

        Action::DumpRegisters => {
        }
    }
}
