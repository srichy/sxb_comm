use clap::{Parser, ValueEnum};
use std::fs::{self, File};
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
        // Do beginning to end
        let parts: Vec<&str> = addr_exp.split('-').collect();
        start = usize::from_str_radix(parts[0], 16).expect("Bad format for starting hex address.");
        count = usize::from_str_radix(parts[1], 16).expect("Bad format for ending hex address.")
            - start + 1;
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

fn parse_filename_and_address(addr_exp: &str) -> Result<(String, usize)> {
    let filename: String;
    let start: usize;

    if addr_exp.contains("@") {
        // Do beginning to end
        let parts: Vec<&str> = addr_exp.split('@').collect();
        filename = String::from(parts[0]);
        start = usize::from_str_radix(parts[1], 16).expect("Bad format for starting hex address.");
        return Ok((filename, start));
    } else {
        return Err(Error::new(ErrorKind::Other, "File upload requires format \"filename@baseaddr\""));
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

fn bin_dump(wdc_device: &mut File, start_addr: usize, mut block_size: usize) -> Result<()> {
    start_cmd(wdc_device, 3)?;
    send_addr(wdc_device, start_addr)?;
    send_count(wdc_device, block_size)?;

    let mut stdout = io::stdout().lock();

    // This following is bonkers.  But I'm not reducing the baud rate, and larger
    // blocks can easily overrun the VIA-based USB serial interface on the WDC board.
    // So I reduced this to a 1-byte buffer (same in the hex-dump) so I can easily
    // open it backup laster to a longer buffer if I deal with setting a slower baud.
    let mut buf = [0; 1];
    while block_size > 0 {
        let n_read = wdc_device.read(&mut buf)?;
        block_size -= n_read;
        stdout.write(&buf)?;
    }

    Ok(())
}

fn bin_upload(wdc_device: &mut File, filename: String, start_addr: usize) -> Result<()> {
    let file_metadata = fs::metadata(&filename)?;
    let file_length = file_metadata.len() as usize;

    if file_length == 0 {
        println!("*** Warning: filename {filename} is zero bytes long.");
        return Ok(());
    }

    let mut in_file = File::open(&filename)?;

    start_cmd(wdc_device, 2)?;
    send_addr(wdc_device, start_addr)?;
    send_count(wdc_device, file_length)?;

    let mut buf = [0; 1];

    loop {
        let n_read = in_file.read(&mut buf)?;
        if n_read == 0 {
            break;
        }
        wdc_device.write(&buf)?;
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
            bin_dump(&mut wdc_device, start_addr, block_size)?;
        }

        Action::WriteBinary => {
            let in_filename: String;
            let start_addr: usize;

            match cli.argument {
                None => {
                    io::stderr().write_all(b"*** filename@hexloc required.\n")?;
                    return Ok(());
                }
                Some(arg) => {
                    (in_filename, start_addr) = parse_filename_and_address(&arg)?;
                }
            }
            let mut wdc_device = File::options().read(true).write(true).open(cli.device)?;
            bin_upload(&mut wdc_device, in_filename, start_addr)?;
        }

        Action::Execute => {
        }
    }
    Ok(())
}
