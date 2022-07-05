mod mips;
mod n64header;
use std::{env, fs::File, io::{Read, Seek}};

use n64header::Endian;
use n64header::ipl3;
use mips::*;


use enum_map::EnumMap;


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

fn parse_entrypoint(data: &[u32]) {
    let mut reg_tracker: EnumMap<MipsGpr, u32> = EnumMap::default();
    let mut branch_reg: MipsGpr = MipsGpr::zero;
    let mut bss_ptr_reg: MipsGpr = MipsGpr::zero;
    let mut bss_sign = 0;
    let bss_size: u32;
    let mut jump_reg: MipsGpr = MipsGpr::zero;
    let jump_addr: u32;
    let mut bss_start: u32;

    let mut consecutive_nops = 0;
    let mut prev_was_branch = false;
    
    for word in data {
        let instr;

        consecutive_nops = if word == &0 { consecutive_nops + 1 } else { 0 };
        if consecutive_nops > 1 {
            println!("Second nop found, breaking out of loop.");
            break;
        }

        print!("/* {word:08X} */");
        print!("{}", " ".repeat(5));
        if prev_was_branch {
            print!(" ");
        }

        instr = disassemble_word(*word).unwrap();
        prev_was_branch = instr.has_delay_slot();

        println!("{}", instr);

        // Register tracking
        match instr {
            MipsInstruction::lui { rDest, imm } => {
                reg_tracker[rDest] = imm << 16;
            }
            MipsInstruction::addiu { rSrc, rDest, imm } => {
                reg_tracker[rDest] = reg_tracker[rSrc] + imm + ((imm & 0x8000) << 1);
            }
            MipsInstruction::addi { imm, .. } => {
                if imm >= 0x8000 { bss_sign -= 1 } else { bss_sign += 1 }
            }
            MipsInstruction::bne { rCmpL, .. } | MipsInstruction::bnez { rCmp: rCmpL, .. } => {
                branch_reg = rCmpL;
            }
            MipsInstruction::jr { rSrc } => {
                jump_reg = rSrc;
            }
            MipsInstruction::sw { rBase, .. } => {
                bss_ptr_reg = rBase;
            }
            _ => (),
        }
    }

    println!();

    jump_addr = reg_tracker[jump_reg];
    bss_size = reg_tracker[branch_reg];
    bss_start = reg_tracker[bss_ptr_reg];
    bss_start = if bss_sign < 0 { bss_start - bss_size } else { bss_start };
    
    println!("jump to:    {:#010X}", jump_addr);
    println!("bss size:   {:#10X}", bss_size);
    println!("bss start:  {:#010X}", bss_start);
    println!("initial sp: {:#010X}", reg_tracker[MipsGpr::sp]);
    // for x in reg_tracker {
    //     println!("{:?}, {:#X}", x.0, x.1);
    // }
}

fn bytes_to_reend_word(bytes: [u8; 4], endian: &Endian) -> u32 {
    match endian {
        Endian::Good => u32::from_be_bytes(bytes),
        Endian::Bad => u32::from_le_bytes(bytes),
        Endian::Ugly => u32::from_be_bytes([bytes[1],bytes[0],bytes[3],bytes[2]]),
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


    let mut buffer = [0u8; 0x40];
    romfile.read_exact(&mut buffer).unwrap();
    reend_array(&mut buffer, &endian);

    
    let header = n64header::read_header(&buffer[..]).expect("Failed to parse header");
    println!();
    println!("ROM Header:");
    println!("{:#}", header);
    
    println!();
    println!("Libultra version: {}", header.libultra_version().unwrap());

    let cic_info = ipl3::identify(romfile).unwrap();

    println!("CIC chip: {}", cic_info.name());
    println!("Corrected entrypoint: {:X}", cic_info.correct_entrypoint(header.entrypoint()));
    // parse_entrypoint(DATA);
}
