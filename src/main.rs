mod mips;
mod n64header;
use std::{env, fs::File, io::{Read, Seek, SeekFrom}};

use mips::disassemble_word;
use n64header::Endian;
use n64header::ipl3;
use n64header::entrypoint;

// const DATA: &[u32] = &[ 
//     0x0C000001,
//     0,
//     0 
// ];
const DATA: &[u32] = &[
    0x3C088004,
    0x2508E940,
    0x24095D50,
    0x2129FFF8,
    0xAD000000,
    0xAD000004,
    0x1520FFFC,
    0x21080008,
    0x3C0A8002,
    0x3C1D8004,
    0x254A5CC0,
    0x01400008,
    0x27BDF330,
    0x00000000,
    0x00000000,
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
        Endian::Bad => [bytes[3],bytes[2],bytes[1],bytes[0]],
        Endian::Ugly => [bytes[1],bytes[0],bytes[3],bytes[2]],
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
    let mut current_rom_address = 0x1000;

    println!();
    println!("Examining up to {:#X} bytes", data.len());
    for chunk in data.chunks_exact(4) {
        let word = bytes_to_reend_word(chunk, endian);
        let instr = disassemble_word(word);
        
        use mips::MipsInstruction;
        match instr {
            Ok(MipsInstruction::b { .. }) => b_count += 1,
            Ok(MipsInstruction::j { .. }) => j_count += 1,
            Err(instr) => {
                println!("Found {}", instr);
                println!("Stopping here and reporting results");
                break;
            }
            _ => (),
        }
        current_rom_address += 1;
    }
    println!("Examined range 0x1000–{current_rom_address:#X} of boot segment");
    println!("  B count:{b_count}");
    println!("  J count:{j_count}");
    println!();
    if b_count > j_count {
        println!("  Probably IDO");
    } else {
        println!("  Probably GCC");
    }
}

fn main() {
    let endian: Endian;
    let args: Vec<String> = env::args().collect();

    let file_name = &args[1];
    let base_name = file_name.split('/').last().expect(format!("Invalid file name: {}", file_name).as_str());
    println!("File: {}", base_name);

    let mut romfile = File::open(file_name).unwrap();

    let file_size = romfile.metadata().unwrap().len();
    println!("ROM size: 0x{file_size:X} bytes ({} MB)", file_size / (1 << 20));

    // Determine endianness
    let mut buffer = [0u8; 4];
    romfile.read_exact(&mut buffer).unwrap();
    endian = n64header::get_endian(&buffer).unwrap();
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
    println!();
    println!("ROM Header:");
    println!("{:#}", header);
    
    println!();
    println!("Libultra version: {}", header.libultra_version().unwrap());

    // Identify ipl3 and correct entrypoint
    let cic_info = ipl3::identify(&romfile).unwrap();
    let entrypoint = cic_info.correct_entrypoint(header.entrypoint());
    println!("CIC chip: {}", cic_info.name());
    println!("Corrected entrypoint: {entrypoint:X}");

    // Parse entrypoint
    let mut buffer = [0u8; 0x100];
    romfile.read(&mut buffer).unwrap();
    let (jump_addr, bss_start, bss_size, initial_sp) = entrypoint::parse(&buffer, entrypoint, &endian);

    println!("jump to:    {:#010X}", jump_addr);
    println!("bss start:  {:#010X}", bss_start);
    println!("bss size:   {:#10X}", bss_size);
    println!("initial sp: {:#010X}", initial_sp);

    // Guess GCC vs IDO
    let boot_size = (bss_start - entrypoint) as usize;
    let mut buffer = vec![0u8; boot_size];
    // let mut buffer = Vec::<u8>::with_capacity(boot_size);
    romfile.seek(SeekFrom::Start(0x1000)).unwrap();
    romfile.read(&mut buffer).unwrap();
    guess_gcc_or_ido(&buffer, &endian);
}
