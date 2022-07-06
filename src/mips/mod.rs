// use std::error::Error;
// use std::{collections::HashMap, fmt::Display};

use enum_map::Enum;
use strum_macros::EnumIter; // 0.17.1
use num_enum::TryFromPrimitive;

use num_traits::Signed;
use std::fmt::{self, Formatter, UpperHex};
// https://stackoverflow.com/a/63607986 , enables printing negative hex literals with a - sign
pub struct ReallySigned<T: PartialOrd + Signed + UpperHex>(pub T);

impl<T: PartialOrd + Signed + UpperHex> UpperHex for ReallySigned<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let prefix = if f.alternate() { "0x" } else { "" };
        let bare_hex = format!("{:X}", self.0.abs());
        f.pad_integral(self.0 >= T::zero(), prefix, &bare_hex)
    }
}

// TODO: implement ns
#[allow(dead_code)]
#[allow(non_camel_case_types)]
pub enum MipsABI {
    o32,
    n32,
    n64,
}

pub struct MipsConfig {
    pub abi: MipsABI,
    pub instruction_print_width: usize,
}
pub static CONFIG: MipsConfig = MipsConfig{
    abi: MipsABI::o32,
    instruction_print_width: 10,
};


// Registers

#[derive(Enum, Clone, Copy, Hash)]
#[allow(non_camel_case_types)]
#[derive(Debug, EnumIter, Eq, PartialEq, TryFromPrimitive)]
#[repr(u32)]
pub enum MipsGpr {
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

// #[derive(Clone)]
struct RegisterInfo {
    name: &'static str,
    clobbered_by_func: bool,
}
impl RegisterInfo {
    const fn new(name: &'static str, clobbered_by_func: bool) -> RegisterInfo {
        RegisterInfo{ name, clobbered_by_func }
    }
}

impl MipsGpr {
    const fn register_info(self) -> RegisterInfo {
        use MipsGpr::*;
        let info = RegisterInfo::new;
        // let reg_info = |name, clobbered_by_func| RegisterInfo{ name, clobbered_by_func };
        match self {
            zero => info("zero", false),
            at   => info("at", true),
            v0   => info("v0", true),
            v1   => info("v1", true),
            a0   => info("a0", true),
            a1   => info("a1", true),
            a2   => info("a2", true),
            a3   => info("a3", true),
            t0   => info("t0", true),
            t1   => info("t1", true),
            t2   => info("t2", true),
            t3   => info("t3", true),
            t4   => info("t4", true),
            t5   => info("t5", true),
            t6   => info("t6", true),
            t7   => info("t7", true),
            s0   => info("s0", false),
            s1   => info("s1", false),
            s2   => info("s2", false),
            s3   => info("s3", false),
            s4   => info("s4", false),
            s5   => info("s5", false),
            s6   => info("s6", false),
            s7   => info("s7", false),
            t8   => info("t8", true),
            t9   => info("t9", true),
            k0   => info("k0", false),
            k1   => info("k1", false),
            gp   => info("gp", false),
            sp   => info("sp", true),
            fp   => info("fp", true),
            ra   => info("ra", false),
        }
    }

    pub const fn name(self) -> &'static str {
        self.register_info().name
    }

    pub const fn clobbered_by_func(self) -> bool {
        self.register_info().clobbered_by_func
    }
}

impl fmt::Display for MipsGpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

// Binary to instruction

enum MipsInstructionFormat {
    Special,
    Regimm,
    J,
    I,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
#[allow(non_camel_case_types)]
#[repr(u32)]
pub enum MipsCPUOp {
    special = 0b000_000,
    regimm  = 0b000_001,
    j       = 0b000_010,
    jal     = 0b000_011,
    beq     = 0b000_100,
    bne     = 0b000_101,
    addi    = 0b001_000,
    addiu   = 0b001_001,
    ori     = 0b001_101,
    lui     = 0b001_111,
    sw      = 0b101_011,
}

impl MipsCPUOp {
    const fn instruction_format(&self) -> MipsInstructionFormat {
        match self {
            MipsCPUOp::special => MipsInstructionFormat::Special,
            MipsCPUOp::regimm => MipsInstructionFormat::Regimm,
            MipsCPUOp::j => MipsInstructionFormat::J,
            MipsCPUOp::jal => MipsInstructionFormat::J,
            MipsCPUOp::beq => MipsInstructionFormat::I,
            MipsCPUOp::bne => MipsInstructionFormat::I,
            MipsCPUOp::addi => MipsInstructionFormat::I,
            MipsCPUOp::addiu => MipsInstructionFormat::I,
            MipsCPUOp::ori => MipsInstructionFormat::I,
            MipsCPUOp::lui => MipsInstructionFormat::I,
            MipsCPUOp::sw => MipsInstructionFormat::I,
        }
    }
}

#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
#[allow(non_camel_case_types)]
#[repr(u32)]
enum MipsCPUFunc {
    jr    = 0b001_000,
}

enum PrintFormat {
    None,
    J(u32),
    R(MipsGpr),
    RR(MipsGpr,MipsGpr),
    RRR(MipsGpr,MipsGpr,MipsGpr),
    I(ReallySigned<i16>),
    RI(MipsGpr,ReallySigned<i16>),
    RRI(MipsGpr,MipsGpr,ReallySigned<i16>),
    RRU(MipsGpr,MipsGpr,u16),
    ROB(MipsGpr,ReallySigned<i16>,MipsGpr),
    Custom(String),
}

impl fmt::Display for PrintFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrintFormat::None => write!(f, ""),
            PrintFormat::J(addr) => write!(f, "{addr}"),
            PrintFormat::R(r1) => write!(f, "{r1}"),
            PrintFormat::RR(r1, r2) => write!(f, "{r1}, {r2}"),
            PrintFormat::RRR(r1, r2, r3) => write!(f, "{r1}, {r2}, {r3}"),
            PrintFormat::I(imm) => write!(f, "{imm:#X}"),
            PrintFormat::RI(r1, imm) => write!(f, "{r1}, {imm:#X}"),
            PrintFormat::RRI(r1, r2, imm) => write!(f, "{r1}, {r2}, {imm:#X}"),
            PrintFormat::RRU(r1, r2, imm) => write!(f, "{r1}, {r2}, {imm:#X}"),
            PrintFormat::ROB(r1, offset, r2) => write!(f, "{r1}, {offset:#X}({r2})"),
            PrintFormat::Custom(string) => write!(f, "{string}"),
        }
    }
}

// Instruction information

#[allow(non_camel_case_types, non_snake_case)]
#[derive(Eq, PartialEq, Hash)]
pub enum MipsInstruction {
    j     {addr: u32},
    jal   {addr: u32},
    beq   {rCmpL: MipsGpr, rCmpR: MipsGpr, offset: u32},
    bne   {rCmpL: MipsGpr, rCmpR: MipsGpr, offset: u32},
    addi  {rSrc: MipsGpr, rDest: MipsGpr, imm: u32},
    addiu {rSrc: MipsGpr, rDest: MipsGpr, imm: u32},
    ori   {rSrc: MipsGpr, rDest: MipsGpr, imm: u32},
    lui   {rDest: MipsGpr, imm: u32},
    sw    {rBase: MipsGpr, rSrc: MipsGpr, offset: u32},
    jr    {rSrc: MipsGpr},

    // Pseudo-instructions
    b     {offset: u32},
    beqz  {rCmp: MipsGpr, offset: u32},
    bnez  {rCmp: MipsGpr, offset: u32},
    nop,

    // Error handling
    unknown {opcode: u32, word: u32},
    invalid {opcode: u32, word: u32},
}

struct MipsInstructionInfo {
    name: &'static str,
    is_branch: bool,
    is_jump: bool,
    print_format: PrintFormat,
}
impl MipsInstructionInfo {
    const fn new(name: &'static str,
    is_branch: bool,
    is_jump: bool,
    print_format: PrintFormat) -> Self {
        MipsInstructionInfo{name, is_branch, is_jump, print_format}
    }
}

impl MipsInstruction {
    fn instruction_info(&self) -> MipsInstructionInfo {
        use MipsInstruction::*;
        let info = MipsInstructionInfo::new;
        match self {
            j     { addr }                 => info("j",    false, true,  PrintFormat::J(*addr)),
            jal   { addr }                 => info("jal",  false, true,  PrintFormat::J(*addr)),
            beq   { rCmpL, rCmpR, offset } => info("bne",  true,  false, PrintFormat::RRI(*rCmpL, *rCmpR, ReallySigned(*offset as i16))),
            bne   { rCmpL, rCmpR, offset } => info("bne",  true,  false, PrintFormat::RRI(*rCmpL, *rCmpR, ReallySigned(*offset as i16))),
            addi  { rDest, rSrc, imm }     => info("addi", false, false, PrintFormat::RRI(*rDest, *rSrc, ReallySigned(*imm as i16))),
            addiu { rDest, rSrc, imm }     => info("addiu", false, false, PrintFormat::RRI(*rDest, *rSrc, ReallySigned(*imm as i16))),
            ori   { rDest, rSrc, imm }     => info("ori",  false, false, PrintFormat::RRU(*rDest, *rSrc, *imm as u16)),
            lui   { rDest, imm }           => info("lui",  false, false, PrintFormat::Custom(format!("{rDest}, ({:#X} >> 16)", imm << 16))),
            sw    {rBase, rSrc, offset}    => info("sw",   false, false, PrintFormat::ROB(*rSrc, ReallySigned(*offset as i16), *rBase)),
            jr    { rSrc }                 => info("jr",   false, true,  PrintFormat::R(*rSrc)),

            // Pseudo-instructions
            b     { offset }               => info("b",    true,  false, PrintFormat::I(ReallySigned(*offset as i16))),
            beqz  { rCmp, offset }         => info("bnez", true,  false, PrintFormat::RI(*rCmp, ReallySigned(*offset as i16))),
            bnez  { rCmp, offset }         => info("bnez", true,  false, PrintFormat::RI(*rCmp, ReallySigned(*offset as i16))),
            nop                            => info("nop",  false, false, PrintFormat::None),

            // Error handling
            unknown { opcode, word }       => info("unknown", false, false, PrintFormat::Custom(format!("(op: {opcode:#08b}, word: {word:08X})"))),
            invalid { opcode, word }       => info("invalid instruction", false, false, PrintFormat::Custom(format!("(op: {opcode:#08b}, word: {word:08X})"))),
        }
    }

    pub fn name(&self) -> &'static str {
        self.instruction_info().name
    }

    pub fn is_branch(&self) -> bool {
        self.instruction_info().is_branch
    }

    pub fn is_jump(&self) -> bool {
        self.instruction_info().is_jump
    }

    pub fn has_delay_slot(&self) -> bool {
        self.is_branch() || self.is_jump()
    }
    
    fn print_format(&self) -> PrintFormat {
        self.instruction_info().print_format
    }
}

impl fmt::Display for MipsInstruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = self.name();
        let width = CONFIG.instruction_print_width;
        write!(f, "{name:<width$} {}", self.print_format())
    }
}

// Disassembly

pub fn disassemble_word(word: u32) -> Result<MipsInstruction, MipsInstruction> {
    let opcode: u32 = word >> 26;
    let opname: Result<MipsCPUOp,_> = opcode.try_into();

    if opname.is_err() {
        return Ok(MipsInstruction::unknown { opcode, word });
    }
    let opname = opname.unwrap();
    let opform = opname.instruction_format();

    // println!("{opname:?}");

    if word == 0 {
        return Ok(MipsInstruction::nop)
    }
    match opform {
        MipsInstructionFormat::Special => {
            let funccode: u32 = word & 0x1F;
            let funcname: Result<MipsCPUFunc,_> = funccode.try_into();
            let rs: MipsGpr = ((word >> 21) & 0x1F).try_into().unwrap();
            let rt: MipsGpr = ((word >> 16) & 0x1F).try_into().unwrap();
            let rd: MipsGpr = ((word >> 10) & 0x1F).try_into().unwrap();
            
            if funcname.is_err() {
                return Ok(MipsInstruction::unknown { opcode, word });
            }
            let funcname = funcname.unwrap();
            match funcname {
                MipsCPUFunc::jr => {
                    // assert_eq!(rt, MipsGpr::zero);
                    // assert_eq!(rd, MipsGpr::zero);
                    if rt == MipsGpr::zero && rd == MipsGpr::zero {
                        Ok(MipsInstruction::jr{ rSrc: rs })
                    } else {
                        Err(MipsInstruction::invalid { opcode, word })
                    }
                }
                // _ => {
                //     Ok(MipsInstruction::unknown { opcode, word })
                // },
            }
        }
        MipsInstructionFormat::I => {
            let rs: MipsGpr = ((word >> 21) & 0x1F).try_into().unwrap();
            let rt: MipsGpr = ((word >> 16) & 0x1F).try_into().unwrap();
            let imm = word & 0xFFFF;
            
            // println!("{imm:X}");
            match opname {
                MipsCPUOp::addi | MipsCPUOp::addiu | MipsCPUOp::ori => {
                    // let imm = imm.try_into()?;
                    match opname {
                        MipsCPUOp::addi => Ok(MipsInstruction::addi{ rSrc: rs, rDest: rt, imm }),
                        MipsCPUOp::addiu => Ok(MipsInstruction::addiu{ rSrc: rs, rDest: rt, imm }),
                        MipsCPUOp::ori => Ok(MipsInstruction::ori{ rSrc: rs, rDest: rt, imm }),
                        _ => Ok(MipsInstruction::unknown { opcode, word }),
                    }
                }
                MipsCPUOp::sw => {
                    let offset = imm;
                    Ok(MipsInstruction::sw{ rBase: rs, rSrc: rt, offset })
                }
                MipsCPUOp::lui => {
                    // assert_eq!(rs, MipsGpr::zero);
                    if rs == MipsGpr::zero {
                        Ok(MipsInstruction::lui{ rDest: rt, imm })
                    } else {
                        Err(MipsInstruction::invalid { opcode, word })
                    }
                }
                MipsCPUOp::beq => {
                    let offset = imm;
                    if rt == MipsGpr::zero {
                        if rs == MipsGpr::zero {
                            Ok(MipsInstruction::b{ offset })
                        } else {
                            Ok(MipsInstruction::beqz{ rCmp: rs, offset })
                        }
                    } else {
                        Ok(MipsInstruction::beq{ rCmpL: rs, rCmpR: rt, offset })
                    }
                }
                MipsCPUOp::bne => {
                    let offset = imm;
                    if rt == MipsGpr::zero {
                        Ok(MipsInstruction::bnez{ rCmp: rs, offset })
                    } else {
                        Ok(MipsInstruction::bne{ rCmpL: rs, rCmpR: rt, offset })
                    }
                }
                _ => Ok(MipsInstruction::unknown { opcode, word }),
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
                _ => Ok(MipsInstruction::unknown { opcode, word }),
            }
        }
        _ => Ok(MipsInstruction::unknown { opcode, word }),
    }
}
