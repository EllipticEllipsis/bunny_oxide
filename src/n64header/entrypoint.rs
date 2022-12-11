use super::super::VERBOSE;
use crate::mips::*;
use crate::n64header::Endian;
use ::rabbitizer;
use enum_map::EnumMap;

fn bytes_to_reend_word(bytes: &[u8], endian: &Endian) -> u32 {
    assert!(bytes.len() >= 4);
    match endian {
        Endian::Good => u32::from_be_bytes(bytes.try_into().unwrap()),
        Endian::Bad => u32::from_le_bytes(bytes.try_into().unwrap()),
        Endian::Ugly => u32::from_be_bytes([bytes[1], bytes[0], bytes[3], bytes[2]]),
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

fn add_signed_imm(u: u32, s: i32) -> u32 {
    if s >= 0 {
        u + s as u32
    } else {
        u - (-s as u32)
    }
}
#[derive(Debug)]
#[allow(non_camel_case_types)]
enum LowerAddrOp {
    None,
    addiu,
    ori,
}

impl Default for LowerAddrOp {
    fn default() -> Self {
        LowerAddrOp::None
    }
}

pub fn parse(data: &[u8], address: u32, endian: &Endian, base_name: &str) -> (u32, u32, u32, u32) {
    let mut reg_tracker: EnumMap<MipsGpr, u32> = EnumMap::default();
    let mut reg_ops: EnumMap<MipsGpr, LowerAddrOp> = EnumMap::default();
    let mut bss_ptr_reg: MipsGpr = MipsGpr::zero;
    let mut bss_size_reg: MipsGpr = MipsGpr::zero;
    let mut bss_size: u32 = 0;
    let mut jump_reg: MipsGpr = MipsGpr::zero;
    let mut jump_addr: Option<u32> = None;
    let mut bss_start: u32;
    let mut jal_found = false;
    let mut final_delay_slot_used = "";

    let mut current_ram_address = address;
    let mut current_rom_address = 0x1000;

    let mut prev_was_jump = false;

    let mut length = 0;

    for (i, chunk) in data.chunks_exact(4).enumerate() {
        let word = bytes_to_reend_word(chunk, endian);

        let instr = rabbitizer::Instruction::new(word, address + 4 * i as u32);
        let my_instruction = MyInstruction { instr };

        // println!("{:?}", my_instruction.instr.instr_id());
        match my_instruction.instr.instr_id() {
            rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_lui => {
                reg_tracker[my_instruction.instr_get_rt()] =
                    (my_instruction.instr.processed_immediate() << 16) as u32;
                // println!("lui: {:#X}", my_instruction.instr.processed_immediate());
            }
            rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_addiu => {
                let out_reg = my_instruction.instr_get_rt();
                // Already applied an addiu
                if reg_tracker[out_reg] & 0xFFFF == 0 {
                    // println!(
                    //     "addiuing: {:#X} + {:#X}",
                    //     reg_tracker[my_instruction.instr_get_rs()],
                    //     my_instruction.instr.processed_immediate()
                    // );
                    reg_tracker[out_reg] = add_signed_imm(
                        reg_tracker[my_instruction.instr_get_rs()],
                        my_instruction.instr.processed_immediate(),
                    );
                    // println!("= {:#X}", reg_tracker[out_reg]);
                    reg_ops[out_reg] = LowerAddrOp::addiu;
                } else {
                    // println!("addiu blocked: {:#X}", reg_tracker[out_reg] & 0xFFFF);
                }
            }
            rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_ori => {
                let out_reg = my_instruction.instr_get_rt();
                reg_tracker[my_instruction.instr_get_rt()] = reg_tracker
                    [my_instruction.instr_get_rs()]
                    | my_instruction.instr.processed_immediate() as u32;
                reg_ops[out_reg] = LowerAddrOp::ori;
            }

            rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_addi => {
                let out_reg = my_instruction.instr_get_rt();
                if my_instruction.instr.processed_immediate() < 0 {
                    bss_size_reg = out_reg;
                }
            }
            rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_sw => {
                let out_reg = my_instruction.instr_get_rs();
                bss_ptr_reg = out_reg;
            }

            rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_jal => {
                jump_addr = Some(my_instruction.instr.instr_index_as_vram());
                jal_found = true;
            }
            rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_jalr
            | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_jr => {
                jump_reg = my_instruction.instr_get_rs();
                jump_addr = Some(reg_tracker[jump_reg]);
            }
            _ => (),
        }

        // Stop after the instruction after the jump
        if prev_was_jump {
            if my_instruction.instr.is_nop() {
                if VERBOSE {
                    println!("Final delay slot NOP. GCC assembler?");
                }
                final_delay_slot_used = "nop";
            } else {
                if VERBOSE {
                    println!("Final delay slot used. IDO assembler?");
                }
                final_delay_slot_used = "yep";
            }
            length = 4 * i;
            break;
        }

        if my_instruction.instr.is_jump() {
            prev_was_jump = true;
        }
    }

    bss_start = reg_tracker[bss_ptr_reg];

    // Work out the rest of the bss stuff
    if bss_size_reg == MipsGpr::zero {
        for (reg, value) in reg_tracker {
            if ![jump_reg, MipsGpr::sp, bss_ptr_reg].contains(&reg) && value != 0 {
                if value < bss_start {
                    bss_size_reg = reg;
                    bss_size = value;
                } else {
                    bss_size = value - bss_start;
                }
                break;
            }
        }
    } else {
        bss_size = reg_tracker[bss_size_reg];
    }

    if jump_addr.is_none() {
        jump_addr = Some(reg_tracker[jump_reg]);
    }

    bss_start = reg_tracker[bss_ptr_reg];

    
    if length > 0x40 {
        eprintln!(
            "{base_name}: Entrypoint is unusually long ({:#X} bytes), recommend closer investigation",
            length
        );
    }
    
    // assert!(bss_start > address);
    
    if !VERBOSE {
        print!(
            "{:X}; {:X}; {:X}; {:X}; {:X}; {:?}; {:?}; {:?}; {}; {}; ",
            length,
            reg_tracker[MipsGpr::sp],
            bss_start,
            bss_size,
            jump_addr.unwrap(),
            reg_ops[MipsGpr::sp],
            reg_ops[bss_ptr_reg],
            reg_ops[bss_size_reg],
            if jal_found { "jal" } else { "jr" },
            final_delay_slot_used
        );
    }

    // println!("jump to:    {:#010X}", jump_addr.unwrap());
    // println!("bss start:  {:#010X}", bss_start);
    // println!("bss size:   {:#10X}", bss_size);
    // println!("initial sp: {:#010X}", reg_tracker[MipsGpr::sp]);
    // for x in reg_tracker {
    //     println!("{:?}, {:#X}", x.0, x.1);
    // }

    (
        jump_addr.unwrap(),
        bss_start,
        bss_size,
        reg_tracker[MipsGpr::sp],
    )
}
