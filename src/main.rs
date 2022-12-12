mod mips;
mod n64header;
use mips::MipsGpr;
use std::{
    env,
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
};

// use mips::disassemble_word;
use n64header::entrypoint;
use n64header::ipl3;
use n64header::Endian;

const VERBOSE: bool = false;

// const DATA: &[u32] = &[
//     0x0C000001,
//     0,
//     0
// ];
const DATA: &[u32] = &[
    0x3C088004, 0x2508E940, 0x24095D50, 0x2129FFF8, 0xAD000000, 0xAD000004, 0x1520FFFC, 0x21080008,
    0x3C0A8002, 0x3C1D8004, 0x254A5CC0, 0x01400008, 0x27BDF330, 0x00000000, 0x00000000,
];

fn bytes_to_reend_word(bytes: &[u8], endian: &Endian) -> u32 {
    assert!(bytes.len() >= 4);
    match endian {
        Endian::Good => u32::from_be_bytes(bytes.try_into().unwrap()),
        Endian::Bad => u32::from_le_bytes(bytes.try_into().unwrap()),
        Endian::Ugly => u32::from_be_bytes([bytes[1], bytes[0], bytes[3], bytes[2]]),
    }
}

fn bytes_to_reend_bytes(bytes: &[u8; 4], endian: &Endian) -> [u8; 4] {
    match endian {
        Endian::Good => *bytes,
        Endian::Bad => [bytes[3], bytes[2], bytes[1], bytes[0]],
        Endian::Ugly => [bytes[1], bytes[0], bytes[3], bytes[2]],
    }
}

/// Re-ends an array in-place
pub fn reend_array(v: &mut [u8], endian: &Endian) {
    let n = v.len();
    assert!(n % 4 == 0);
    match endian {
        Endian::Good => (),
        Endian::Bad => {
            for chunk in v.chunks_exact_mut(4) {
                chunk.reverse();
            }
        }
        Endian::Ugly => {
            for chunk in v.chunks_exact_mut(2) {
                chunk.reverse();
            }
        }
    };
}

fn guess_gcc_or_ido(data: &[u8], endian: &Endian) {
    let mut j_count = 0;
    let mut b_count = 0;

    if VERBOSE {
        println!();
        println!("Examining up to {:#X} bytes", data.len());
    }

    let mut in_function = false;
    let mut text_end = None;
    // let mut consecutive_nops = 0;
    for (i, chunk) in data.chunks_exact(4).rev().enumerate() {
        let word = bytes_to_reend_word(chunk, endian);
        let instr = rabbitizer::Instruction::new(word, 0);

        // if instr.is_nop() {
        //     consecutive_nops += 1;
        // } else {
        //     consecutive_nops = 0;
        // }
        if instr.is_jr_ra() {
            in_function = true;
            if text_end.is_none() {
                text_end = Some(0x1000 + data.len() - 4 * i);
            }
        } else if !instr.is_valid() {
            in_function = false
        }

        if in_function {
            match instr.instr_id() {
                rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_b => b_count += 1,
                rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_j => j_count += 1,
                _ => (),
            }
        }
    }

    if VERBOSE {
        println!(
            "Examined range 0x1000–{:#X} of boot segment",
            text_end.unwrap()
        );
        println!("  B count:{b_count}");
        println!("  J count:{j_count}");
        println!();
        if b_count + j_count < 100 {
            println!("  Not enough to guess compiler");
        } else if b_count > j_count {
            println!("  Probably IDO");
        } else {
            println!("  Probably GCC");
        }
    } else {
        print!("{:#X}; {b_count}; {j_count}; ", text_end.unwrap());
    }
}

struct MyInstruction {
    instr: rabbitizer::Instruction,
}

impl MyInstruction {
    fn instr_get_rs(&self) -> MipsGpr {
        ((self.instr.raw() >> 21) & 0x1F).try_into().unwrap()
    }
    fn instr_get_rt(&self) -> MipsGpr {
        ((self.instr.raw() >> 16) & 0x1F).try_into().unwrap()
    }
    fn instr_get_rd(&self) -> MipsGpr {
        ((self.instr.raw() >> 11) & 0x1F).try_into().unwrap()
    }
}

fn run(file_name: &String) -> Result<(), String> {
    let base_name = file_name
        .split('/')
        .last()
        .expect(format!("Invalid file name: {}", file_name).as_str());

    if VERBOSE {
        println!("File: {base_name}");
    } else {
        print!("{base_name}; ");
    }

    let mut romfile = File::open(file_name).unwrap();

    let file_size = romfile.metadata().unwrap().len();
    if VERBOSE {
        println!(
            "ROM size: 0x{file_size:X} bytes ({} MB)",
            file_size / (1 << 20)
        );
    } else {
        print!("{file_size:X}; ");
    }

    // Determine endianness
    let mut buffer = [0u8; 4];
    romfile.read_exact(&mut buffer).unwrap();
    let endian = n64header::get_endian(&buffer).unwrap();
    // match endian {
    //     Endian::Good => {
    //         println!("Endian: {endian:?}");
    //     }
    //     _ => unimplemented!("Wordswapped and byteswapped ROMs are not currently supported")
    // }
    romfile.rewind().unwrap();

    // Read header
    let mut buffer = [0u8; 0x40];
    romfile.read_exact(&mut buffer).unwrap();
    reend_array(&mut buffer, &endian);
    let header = n64header::read_header(&buffer[..]).expect("Failed to parse header");
    if VERBOSE {
        println!();
        println!("ROM Header:");
        println!("{:#}", header);
        println!();
        println!("Libultra version: {}", header.libultra_version().unwrap());
    } else {
        print!("{}; {}; ", header, header.libultra_version().unwrap());
    }

    // Identify ipl3 and correct entrypoint
    let cic_info = ipl3::identify(&romfile).unwrap();
    let entrypoint = cic_info.correct_entrypoint(header.entrypoint());
    if VERBOSE {
        println!("CIC chip: {}", cic_info.name());
        println!("Corrected entrypoint: {entrypoint:X}");
    } else {
        print!("{}; ", cic_info.name());
        print!("{entrypoint:X}; ");
    }

    // Parse entrypoint
    let mut buffer = [0u8; 0x100];
    romfile.read(&mut buffer).unwrap();
    let (jump_addr, bss_start, bss_size, initial_sp) =
        entrypoint::parse(&buffer, entrypoint, &endian, base_name);

    if VERBOSE {
        println!("jump to:    {:#010X}", jump_addr);
        println!("bss start:  {:#010X}", bss_start);
        println!("bss size:   {:#10X}", bss_size);
        println!("initial sp: {:#010X}", initial_sp);
    }

    // Guess GCC vs IDO
    let mut buffer;
    if bss_start > entrypoint {
        let boot_size = (bss_start - entrypoint) as usize;
        buffer = vec![0u8; boot_size];
    } else {
        buffer = vec![0u8; 0x100000];
    }
    romfile.seek(SeekFrom::Start(0x1000)).unwrap();
    romfile.read(&mut buffer).unwrap();
    guess_gcc_or_ido(&buffer, &endian);

    if !VERBOSE {
        println!();
    }

    Ok(())
}

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("USAGE: {} ROMFILE", &args[0]);
        return Err("Not enough arguments".to_string());
    }

    // let file_name = &args[1];

    // run(file_name);

    let mut i = 1;
    while i < args.len() {
        eprintln!("{}", args[i]);
        run(&args[i])?;
        io::stdout().flush().unwrap();
        i += 1;
    }
    Ok(())
}
