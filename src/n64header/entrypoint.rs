use crate::mips::*;
use crate::n64header::Endian;
use enum_map::EnumMap;

fn bytes_to_reend_word(bytes: &[u8], endian: &Endian) -> u32 {
    assert!(bytes.len() >= 4);
    match endian {
        Endian::Good => u32::from_be_bytes(bytes.try_into().unwrap()),
        Endian::Bad => u32::from_le_bytes(bytes.try_into().unwrap()),
        Endian::Ugly => u32::from_be_bytes([bytes[1], bytes[0], bytes[3], bytes[2]]),
    }
}

pub fn parse(data: &[u8], address: u32, endian: &Endian) -> (u32, u32, u32, u32) {
    let mut reg_tracker: EnumMap<MipsGpr, u32> = EnumMap::default();
    let mut branch_reg: MipsGpr = MipsGpr::zero;
    let mut bss_ptr_reg: MipsGpr = MipsGpr::zero;
    let mut bss_sign = 0;
    let bss_size: u32;
    let mut jump_reg: MipsGpr = MipsGpr::zero;
    let jump_addr: u32;
    let mut bss_start: u32;

    let mut consecutive_nops = 0;
    let mut prev_has_delay_slot = false;

    let mut current_ram_address = address;
    let mut current_rom_address = 0x1000;

    for chunk in data.chunks_exact(4) {
        let word = bytes_to_reend_word(chunk, endian);

        let instr;

        consecutive_nops = if word == 0 { consecutive_nops + 1 } else { 0 };
        if consecutive_nops > 1 {
            println!("Second nop found, breaking out of loop.");
            break;
        }

        print!("/* {current_ram_address:08X} {current_rom_address:06X} {word:08X} */");
        print!("{}", " ".repeat(5));
        if prev_has_delay_slot {
            print!(" ");
        }

        instr = disassemble_word(word).unwrap_or_else(|_| panic!("{word:X}"));
        prev_has_delay_slot = instr.has_delay_slot();

        println!("{}", instr);

        // Register tracking
        match instr {
            MipsInstruction::lui { rDest, imm } => {
                reg_tracker[rDest] = imm << 16;
            }
            MipsInstruction::addiu { rSrc, rDest, imm } => {
                reg_tracker[rDest] = reg_tracker[rSrc] + imm + ((imm & 0x8000) << 1);
            }
            MipsInstruction::ori { rSrc, rDest, imm } => {
                reg_tracker[rDest] = reg_tracker[rSrc] + imm;
            }
            MipsInstruction::addi { imm, .. } => {
                if imm >= 0x8000 {
                    bss_sign -= 1
                } else {
                    bss_sign += 1
                }
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

        current_ram_address += 4;
        current_rom_address += 4;
    }

    println!();

    jump_addr = reg_tracker[jump_reg];
    bss_size = reg_tracker[branch_reg];
    bss_start = reg_tracker[bss_ptr_reg];
    bss_start = if bss_sign < 0 {
        bss_start - bss_size
    } else {
        bss_start
    };

    // println!("jump to:    {:#010X}", jump_addr);
    // println!("bss start:  {:#010X}", bss_start);
    // println!("bss size:   {:#10X}", bss_size);
    // println!("initial sp: {:#010X}", reg_tracker[MipsGpr::sp]);
    // for x in reg_tracker {
    //     println!("{:?}, {:#X}", x.0, x.1);
    // }
    (jump_addr, bss_start, bss_size, reg_tracker[MipsGpr::sp])
}
