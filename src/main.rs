use clap::{Parser, ValueEnum};
use std::fs::File;
use std::io::{self, Read, Write, Error, ErrorKind, Result};

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[arg(short, long, help="Serial device pathname")]
    device: String,

    #[arg(short, long, value_enum)]
    action: Action,

    #[arg(help="Filename or hex address depending on action")]
    argument: Option<String>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Action {
    HexDump,
    ReadBinary,
    WriteBinary,
    Execute,
}

fn start_cmd(wdc_dev: &mut File, cmd: u8) -> Result<()> {
    let mut buf: [u8; 2] = [0x55, 0xaa];
    wdc_dev.write_all(&buf)?;
    wdc_dev.read_exact(&mut buf[0..1])?;
    if buf[0] != 0xcc {
        return Err(Error::new(ErrorKind::Other, "Unexpected status from WDC device"));
    }
    buf[0] = cmd;
    wdc_dev.write_all(&buf[0..1])?;
    Ok(())
}

fn send_addr(wdc_dev: &mut File, start_addr: usize) -> Result<()> {
    let buf: [u8; 3] = [
        (start_addr & 0xff) as u8,
        ((start_addr >> 8) & 0xff) as u8,
        ((start_addr >> 16) & 0xff) as u8,
    ];
    wdc_dev.write_all(&buf)?;
    Ok(())
}

fn send_count(wdc_dev: &mut File, block_size: usize) -> Result<()> {
    let buf: [u8; 2] = [
        (block_size & 0xff) as u8,
        ((block_size >> 8) & 0xff) as u8,
    ];
    wdc_dev.write_all(&buf)?;
    Ok(())
}

fn parse_address_expr(addr_exp: &str) -> (usize, usize) {
    let start: usize;
    let count: usize;

    if addr_exp.contains("-") {
        // Do base + length
        let parts: Vec<&str> = addr_exp.split('-').collect();
        start = usize::from_str_radix(parts[0], 16).expect("Bad format for starting hex address.");
        count = parts[1].parse::<usize>().expect("Bad format for ending hex address.") - start + 1;
        return (start, count);
    } else if addr_exp.contains(",") {
        // Do base + length
        let parts: Vec<&str> = addr_exp.split(',').collect();
        start = usize::from_str_radix(parts[0], 16).expect("Bad format for hex starting address.");
        count = parts[1].parse::<usize>().expect("Bad format for block size.");
        return (start, count);
    } else {
        start = addr_exp.parse::<usize>().expect("Bad format for address.");
        return (start, 1);
    }
}

fn hex_dump(wdc_device: &mut File, start_addr: usize, mut block_size: usize) -> Result<()> {
    start_cmd(wdc_device, 3)?;
    send_addr(wdc_device, start_addr)?;
    send_count(wdc_device, block_size)?;

    let mut buf = [0; 1];
    let mut offset: usize = 0;
    let mut line_offset: usize = 0;
    let mut ascii_line = String::new();
    while block_size > 0 {
        let n_read = wdc_device.read(&mut buf)?;
        block_size -= n_read;

        line_offset = offset % 16;

        if line_offset == 0 {
            print!("{:06x} ", start_addr + offset);
            ascii_line.clear();
        }

        let ch = buf[0] as char;
        if ch.is_ascii_graphic() || ch == ' ' {
            ascii_line.push(buf[0] as char);
        } else {
            ascii_line.push('.');
        }
        print!("{:02x}", buf[0]);

        if line_offset == 7 {
            print!("-");
        } else {
            if line_offset == 15 {
                print!(" : {ascii_line}\n");
            } else {
                print!(" ");
            }
        }

        offset += n_read;
    }
    if line_offset != 15 {
        for i in (line_offset+1)..16 {
            print!("  ");
            if i == 7 {
                print!("-");
            } else {
                if i == 15 {
                    print!(" : {ascii_line}\n");
                } else {
                    print!(" ");
                }
            }
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    let cli = Args::parse();

    match cli.action {

        Action::HexDump => {
            let start_addr: usize;
            let block_size: usize;

            match cli.argument {
                None => {
                    io::stderr().write_all(b"*** Address or Address range required.\n")?;
                    return Ok(());
                }
                Some(arg) => {
                    (start_addr, block_size) = parse_address_expr(&arg);
                }
            }

            let mut wdc_device = File::options().read(true).write(true).open(cli.device)?;
            hex_dump(&mut wdc_device, start_addr, block_size)?;
        }

        Action::ReadBinary => {
            match cli.argument {
                None => {
                    io::stderr().write_all(b"*** Binary file name required.\n")?;
                    return Ok(());
                }
                Some(arg) => {
                    println!("@@@ Good job.  You specified {arg}.")
                }
            }
            let mut wdc_device = File::options().read(true).write(true).open(cli.device)?;
            start_cmd(&mut wdc_device, 3)?;
        }

        Action::WriteBinary => {
            match cli.argument {
                None => {
                    io::stderr().write_all(b"*** Binary file name required.\n")?;
                    return Ok(());
                }
                Some(arg) => {
                    println!("@@@ Good job.  You specified {arg}.")
                }
            }
            let mut wdc_device = File::options().read(true).write(true).open(cli.device)?;
            start_cmd(&mut wdc_device, 2)?;
        }

        Action::Execute => {
        }
    }
    Ok(())
}
