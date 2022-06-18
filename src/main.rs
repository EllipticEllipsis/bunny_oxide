use std::error::Error;
// use strum::IntoEnumIterator; // 0.17.1
use strum_macros::EnumIter; // 0.17.1

use num_enum::TryFromPrimitive;
// use std::convert::TryInto;
// use std::fmt;

// use std::collections::{HashMap, HashSet};

use std::fmt::{self, Formatter, UpperHex};
use num_traits::Signed;

use enum_map::{Enum, EnumMap};

// https://stackoverflow.com/a/63607986 , enables printing negative hex literals with a - sign
struct ReallySigned<T: PartialOrd + Signed + UpperHex>(T);

impl<T: PartialOrd + Signed + UpperHex> UpperHex for ReallySigned<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let prefix = if f.alternate() { "0x" } else { "" };
        let bare_hex = format!("{:X}", self.0.abs());
        f.pad_integral(self.0 >= T::zero(), prefix, &bare_hex)
    }
}

#[derive(Enum, Clone, Copy, Hash)]
#[allow(non_camel_case_types)]
#[derive(Debug, EnumIter, Eq, PartialEq, TryFromPrimitive)]
#[repr(u32)]
enum MipsGpr {
    zero = 0,
    at =   1,
    v0 =   2,
    v1 =   3,
    a0 =   4,
    a1 =   5,
    a2 =   6,
    a3 =   7,
    t0 =   8,
    t1 =   9,
    t2 =  10,
    t3 =  11,
    t4 =  12,
    t5 =  13,
    t6 =  14,
    t7 =  15,
    s0 =  16,
    s1 =  17,
    s2 =  18,
    s3 =  19,
    s4 =  20,
    s5 =  21,
    s6 =  22,
    s7 =  23,
    t8 =  24,
    t9 =  25,
    k0 =  26,
    k1 =  27,
    gp =  28,
    sp =  29,
    fp =  30,
    ra =  31,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
#[allow(non_camel_case_types)]
#[repr(u32)]
enum MipsCPUOp {
    special = 0b000_000,
    regimm  = 0b000_001,
    j       = 0b000_010,
    jal     = 0b000_011,
    bne     = 0b000_101,
    addi    = 0b001_000,
    addiu   = 0b001_001,
    lui     = 0b001_111,
    sw      = 0b101_011,
}

#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
#[allow(non_camel_case_types)]
#[repr(u32)]
enum MipsCPUFunc {
    jr    = 0b001_000,
    nop   = 0b10_000000,
}

#[allow(non_camel_case_types)]
#[derive(Eq, PartialEq, Hash)]
enum MipsInstruction {
    j     {addr: u32},
    jal   {addr: u32},
    bne   {rs: MipsGpr, rt: MipsGpr, offset: u32},
    addi  {rs: MipsGpr, rt: MipsGpr, imm: u32},
    addiu {rs: MipsGpr, rt: MipsGpr, imm: u32},
    lui   {rt: MipsGpr, imm: u32},
    sw    {base: MipsGpr, rt: MipsGpr, offset: u32},
    jr    {rs: MipsGpr},
    // Pseudo-instructions
    bnez  {rs: MipsGpr, offset: u32},
    nop,
}

enum MipsInstructionFormat {
    Special,
    Regimm,
    J,
    I,
}

impl MipsCPUOp {
    fn instruction_format(&self) -> MipsInstructionFormat {
        match self {
            MipsCPUOp::special => MipsInstructionFormat::Special,
            MipsCPUOp::j => MipsInstructionFormat::J,
            MipsCPUOp::jal => MipsInstructionFormat::J,
            MipsCPUOp::bne => MipsInstructionFormat::I,
            MipsCPUOp::addi => MipsInstructionFormat::I,
            MipsCPUOp::addiu => MipsInstructionFormat::I,
            MipsCPUOp::lui => MipsInstructionFormat::I,
            MipsCPUOp::sw => MipsInstructionFormat::I,
            _ => unimplemented!(),
        }
    }
    // const mips_op_to_instruction: HashMap<MipsCPUOp, MipsInstruction> = HashMap::from([
    //     (MipsCPUOp::addi , MipsInstruction::addi),
    //     (MipsCPUOp::addiu , MipsInstruction::addiu),
    // ]);

    
}

impl fmt::Display for MipsInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
        MipsInstruction::j     { addr } => write!(f, "j {addr:#010X}"),
        MipsInstruction::jal   { addr } => write!(f, "jal {addr:#010X}"),
        MipsInstruction::bne   { rs, rt, offset } => write!(f, "bne {rs:?}, {rt:?}, {:#X}", ReallySigned(*offset as i16)),
        MipsInstruction::addi  { rs, rt, imm } => write!(f, "addi {rs:?}, {rt:?}, {:#X}", ReallySigned(*imm as i16)),
        MipsInstruction::addiu { rs, rt, imm } => write!(f, "addiu {rs:?}, {rt:?}, {:#X}", ReallySigned(*imm as i16)),
        MipsInstruction::lui   { rt, imm } => write!(f, "lui {rt:?}, ({:#X} >> 16)", imm << 16),
        MipsInstruction::sw    { base, rt, offset } => write!(f, "sw {rt:?}, {:#X}({base:?})", ReallySigned(*offset as i16)),
        MipsInstruction::jr    { rs } => write!(f, "jr {rs:?}"),
        // Pseudo-instructions
        MipsInstruction::bnez {rs, offset } => write!(f, "bnez {rs:?}, {:#X}", ReallySigned(*offset as i16)),
        MipsInstruction::nop => write!(f, "nop"),
        _ => unimplemented!(),
        }
    }
}

impl MipsInstruction {
    fn is_branch(&self) -> bool {
        matches!(self, MipsInstruction::j{..} | MipsInstruction::jal{..} | MipsInstruction::jr{..} | MipsInstruction::bne{..} | MipsInstruction::bnez{..})
    }
}

fn disassemble_word(word: u32) -> Result<MipsInstruction, Box<dyn Error>> {
    let opcode: u32 = word >> 26;
    let opname: MipsCPUOp = opcode.try_into()?;
    let opform = opname.instruction_format();

    if word == 0 {
        return Ok(MipsInstruction::nop)
    }
    match opform {
        MipsInstructionFormat::Special => {
            let funccode: u32 = word & 0x1F;
            let funcname: MipsCPUFunc = funccode.try_into()?;
            let rs: MipsGpr = ((word >> 21) & 0x1F).try_into()?;
            let rt: MipsGpr = ((word >> 16) & 0x1F).try_into()?;
            let rd: MipsGpr = ((word >> 10) & 0x1F).try_into()?;

            match funcname {
                MipsCPUFunc::jr => {
                    assert_eq!(rt, MipsGpr::zero);
                    assert_eq!(rd, MipsGpr::zero);
                    Ok(MipsInstruction::jr{ rs })
                }
                _ => unimplemented!(),
            }
        }
        MipsInstructionFormat::I => {
            let rs: MipsGpr = ((word >> 21) & 0x1F).try_into()?;
            let rt: MipsGpr = ((word >> 16) & 0x1F).try_into()?;
            let imm = word & 0xFFFF;
            
            // println!("{imm:X}");
            match opname {
                MipsCPUOp::addi | MipsCPUOp::addiu => {
                    let imm = imm.try_into()?;
                    match opname {
                        MipsCPUOp::addi => Ok(MipsInstruction::addi{ rs, rt, imm }),
                        MipsCPUOp::addiu => Ok(MipsInstruction::addiu{ rs, rt, imm }),
                        _ => unimplemented!(),
                    }
                }
                MipsCPUOp::sw => {
                    let offset = imm.try_into()?;
                    let base = rs;
                    Ok(MipsInstruction::sw{ base, rt, offset })
                }
                MipsCPUOp::lui => {
                    // let imm = imm << 16;
                    assert_eq!(rs, MipsGpr::zero);
                    Ok(MipsInstruction::lui{ rt, imm })
                }
                MipsCPUOp::bne => {
                    let offset = imm.try_into()?;
                    if rt == MipsGpr::zero {
                        Ok(MipsInstruction::bnez{ rs, offset })
                    } else {
                        Ok(MipsInstruction::bne{ rs, rt, offset })
                    }
                }
                _ => unimplemented!("Unsupported opcode {opcode:#08b} (read from {word:#010X})"),
            }
        }
        MipsInstructionFormat::J => {
            let addr = (word & 0x3FFFFFF) << 2;

            match opname {
                MipsCPUOp::j => {
                    Ok(MipsInstruction::j{ addr })
                }
                MipsCPUOp::jal => {
                    Ok(MipsInstruction::jal{ addr })
                }
                _ => unimplemented!("Unsupported opcode {opcode:#08b} (read from {word:#010X})"),
            }
        }
        _ => unimplemented!("Unsupported opcode {opcode:#08b} (read from {word:#010X})"),
    }
}


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



fn main() {
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
    
    for word in DATA {
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
        prev_was_branch = instr.is_branch();

        println!("{}", instr);

        // Register tracking
        match instr {
            MipsInstruction::lui { rt, imm } => {
                reg_tracker[rt] = imm << 16;
            }
            MipsInstruction::addiu { rs, rt, imm } => {
                reg_tracker[rt] = reg_tracker[rs] + imm + ((imm & 0x8000) << 1);
            }
            MipsInstruction::addi { imm, .. } => {
                if imm >= 0x8000 { bss_sign -= 1 } else { bss_sign += 1 }
            }
            MipsInstruction::bne { rs, .. } | MipsInstruction::bnez { rs, .. } => {
                branch_reg = rs;
            }
            MipsInstruction::jr { rs } => {
                jump_reg = rs;
            }
            MipsInstruction::sw { base, .. } => {
                bss_ptr_reg = base;
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
