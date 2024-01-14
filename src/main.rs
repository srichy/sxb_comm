use clap::{Parser, ValueEnum};
use std::fs::File;
use std::io::{Read, Write, Error, ErrorKind, Result};

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[arg(short, long, help="Serial device pathname")]
    device: String,

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

fn startcmd(wdc_dev: &mut File, cmd: u8) -> Result<()> {
    let mut write_buf: [u8; 2] = [0x55, 0xaa];
    let mut read_buf: [u8; 1] = [0];
    wdc_dev.write_all(&write_buf)?;
    wdc_dev.read_exact(&mut read_buf)?;
    if read_buf[0] != 0xcc {
        return Err(Error::new(ErrorKind::Other, "Unexpected status from WDC device"));
    }
    write_buf[0] = cmd;
    wdc_dev.write_all(&write_buf[0..1])?;
    Ok(())
}

fn main() -> Result<()> {
    let cli = Args::parse();

    let mut wdc_device = File::options().read(true).write(true).open(cli.device)?;

    match cli.action {

        Action::HexDump => {
            startcmd(&mut wdc_device, 3)?;
        }

        Action::ReadBinary => {
            startcmd(&mut wdc_device, 3)?;
        }

        Action::WriteBinary => {
            startcmd(&mut wdc_device, 2)?;
        }

        Action::Execute => {
        }

        Action::DumpRegisters => {
        }
    }
    Ok(())
}
