use clap::{Parser, ValueEnum};
use serialport::{TTYPort, FlowControl, DataBits, Parity, StopBits};
use std::fs::{self, File};
use std::io::{self, Read, Write, Error, ErrorKind, Result};
use std::thread::sleep;
use std::time::Duration;

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
    Execute6502,
    Execute65816,
    SExecute6502,
    SExecute65816,
}

fn open_dev(dev_path: &str) -> Result<TTYPort> {
    println!("Opening {dev_path}...");
    let wdc_dev = serialport::new(dev_path, 57600)
        .flow_control(FlowControl::Hardware)
        .timeout(Duration::from_millis(100))
        .data_bits(DataBits::Eight)
        .stop_bits(StopBits::One)
        .parity(Parity::None)
        .open_native()?;
    sleep(Duration::from_millis(50));
    //sync(&mut wdc_dev)?;
    //sync(&mut wdc_dev)?;
    //sync(&mut wdc_dev)?;
    Ok(wdc_dev)
}

pub fn sync(wdc_dev: &mut TTYPort) -> Result<()> {
    let mut retry_count: u8 = 0;
    loop {
        println!("Syncing ... {retry_count}");
        let mut buf: [u8; 1] = [0];
        let r = wdc_dev.write(&buf[0..1])?;
        if r != 1 {
            println!("@@@ Cannot write single byte.");
            continue;
        }
        println!("Sync waiting for zero byte...");
        let r = wdc_dev.read(&mut buf[0..1])?;
        if r == 0 {
            retry_count += 1;
            if retry_count > 5 {
                return Err(Error::new(ErrorKind::Other, "Cannot sync"));
            }
            println!("@@@ Sync read 0 bytes.  Retrying.");
        }
        if buf[0] != 0 {
            let byte = buf[0];
            println!("*** [sync] Bad response: {byte}");
        } else {
            println!("Sync good.");
            break;
        }
    }
    Ok(())
}

fn start_cmd(wdc_dev: &mut TTYPort, cmd: u8) -> Result<()> {
    loop {
        let mut buf: [u8; 2] = [0x55, 0xaa];
        wdc_dev.write_all(&buf[0..2])?;
        wdc_dev.read_exact(&mut buf[0..1])?;
        if buf[0] != 0xcc {
            let byte = buf[0];
            println!("Byte: {byte}");
            sync(wdc_dev)?;
            continue;
        }
        buf[0] = cmd;
        wdc_dev.write_all(&buf[0..1])?;
        break;
    }
    sleep(Duration::from_millis(5));
    Ok(())
}

fn send_addr(wdc_dev: &mut TTYPort, start_addr: usize) -> Result<()> {
    let buf: [u8; 3] = [
        (start_addr & 0xff) as u8,
        ((start_addr >> 8) & 0xff) as u8,
        ((start_addr >> 16) & 0xff) as u8,
    ];
    wdc_dev.write_all(&buf)?;
    sleep(Duration::from_millis(5));
    Ok(())
}

fn send_count(wdc_dev: &mut TTYPort, block_size: usize) -> Result<()> {
    let buf: [u8; 2] = [
        (block_size & 0xff) as u8,
        ((block_size >> 8) & 0xff) as u8,
    ];
    wdc_dev.write_all(&buf)?;
    sleep(Duration::from_millis(5));
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

fn hex_dump(wdc_dev: &mut TTYPort, start_addr: usize, mut block_size: usize) -> Result<()> {
    start_cmd(wdc_dev, 3)?;
    send_addr(wdc_dev, start_addr)?;
    send_count(wdc_dev, block_size)?;

    let mut buf = [0; 1];
    let mut offset: usize = 0;
    let mut line_offset: usize = 0;
    let mut ascii_line = String::new();
    while block_size > 0 {
        let n_read = wdc_dev.read(&mut buf)?;
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
            print!("  ");
        } else {
            if line_offset == 15 {
                print!("  |{ascii_line}|\n");
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
                print!("  ");
            } else {
                if i == 15 {
                    print!("  |{ascii_line}|\n");
                } else {
                    print!(" ");
                }
            }
        }
    }

    Ok(())
}

fn bin_dump(wdc_dev: &mut TTYPort, start_addr: usize, mut block_size: usize) -> Result<()> {
    start_cmd(wdc_dev, 3)?;
    send_addr(wdc_dev, start_addr)?;
    send_count(wdc_dev, block_size)?;

    let mut stdout = io::stdout().lock();

    // This following is bonkers.  But I'm not reducing the baud rate, and larger
    // blocks can easily overrun the VIA-based USB serial interface on the WDC board.
    // So I reduced this to a 1-byte buffer (same in the hex-dump) so I can easily
    // open it backup laster to a longer buffer if I deal with setting a slower baud.
    let mut buf = [0; 1];
    while block_size > 0 {
        let n_read = wdc_dev.read(&mut buf)?;
        block_size -= n_read;
        stdout.write(&buf)?;
    }

    Ok(())
}

fn bin_upload(wdc_dev: &mut TTYPort, filename: String, start_addr: usize) -> Result<()> {
    let file_metadata = fs::metadata(&filename)?;
    let file_length = file_metadata.len() as usize;

    if file_length == 0 {
        println!("*** Warning: filename {filename} is zero bytes long.");
        return Ok(());
    }
    println!("Sending {file_length} bytes to {start_addr:x} address...");

    let mut in_file = File::open(&filename)?;

    start_cmd(wdc_dev, 2)?;
    send_addr(wdc_dev, start_addr)?;
    send_count(wdc_dev, file_length)?;

    let mut buf = [0; 1];
    let mut byte_count = 0;
    loop {
        let n_read = in_file.read(&mut buf)?;
        if n_read == 0 {
            break;
        }
        wdc_dev.write_all(&buf)?;
        sleep(Duration::from_millis(1));
        byte_count +=1 ;
        if byte_count % 100 == 0 {
            println!("\r{byte_count}");
        }
    }

    println!("Wrote {byte_count} bytes.");

    Ok(())
}

fn send_execute(cli: &Args, cpu_mode: u8, do_serial: bool) -> Result<()> {
    let entry_addr: usize;

    match &cli.argument {
        None => {
            io::stderr().write_all(b"*** Entry point address required.\n")?;
            return Ok(());
        }
        Some(arg) => {
            entry_addr = usize::from_str_radix(arg, 16).expect("Entry address must be hex.");
        }
    }

    // Exec record offsets from 0x7e00, little-endian, all encoded as 16-bit
    // values even if targeting 6502 mode.  Note that I am currently not
    // handling any of these other than processor mode and entry address and
    // am otherwise setting everything up as a typical 6502 execution environment
    // (same as a power-on reset of a 65816, or 65c02).  I may want to think about
    // setting the interrupt disable flag in the processor status register...
    // ---------------------------------------------------------------------
    // 0,1: A, B (C)
    // 2,3: X
    // 4,5: Y
    // 6,7: START/entry address
    // 8,9: direct page base address
    // a,b: stack pointer base address
    // c  : processor status register
    // d  : proc mode: 0 -> 65816, 1 -> 6502
    // e  : program bank register
    // f  : data bank register

    let mut buf = [0; 16];
    buf[6] = (entry_addr & 0xff) as u8;
    buf[7] = ((entry_addr >> 8) & 0xff) as u8;
    buf[10] = 255;              // stack pointer (lo)
    buf[11] = 1;                // stack pointer (hi)
    buf[13] = cpu_mode;

    let mut wdc_dev = open_dev(&cli.device)?;
    start_cmd(&mut wdc_dev, 2)?;
    send_addr(&mut wdc_dev, 0x007e00)?;
    send_count(&mut wdc_dev, buf.len())?;

    for i in 0..buf.len() {
        wdc_dev.write_all(&buf[i..i+1])?;
        sleep(Duration::from_millis(5));
    }
    start_cmd(&mut wdc_dev, 5)?;

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

            let mut wdc_dev = open_dev(&cli.device)?;
            hex_dump(&mut wdc_dev, start_addr, block_size)?;
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
            let mut wdc_dev = open_dev(&cli.device)?;
            bin_dump(&mut wdc_dev, start_addr, block_size)?;
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
            let mut wdc_dev = open_dev(&cli.device)?;
            sleep(Duration::from_millis(50));
            bin_upload(&mut wdc_dev, in_filename, start_addr)?;
        }

        Action::Execute6502 => {
            // Store resume context to 0x7e00, then call command/function 5 (or 6?)
            send_execute(&cli, 1, false)?;
        }

        Action::Execute65816 => {
            // Store resume context to 0x7e00, then call command/function 5 (or 6?)
            send_execute(&cli, 0, false)?;
        }
        Action::SExecute6502 => {
            // Store resume context to 0x7e00, then call command/function 5 (or 6?)
            send_execute(&cli, 1, true)?;
        }

        Action::SExecute65816 => {
            // Store resume context to 0x7e00, then call command/function 5 (or 6?)
            send_execute(&cli, 0, true)?;
        }
    }
    Ok(())
}
