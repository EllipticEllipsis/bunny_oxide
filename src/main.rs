// use strum::IntoEnumIterator; // 0.17.1
use strum_macros::EnumIter; // 0.17.1

use num_enum::TryFromPrimitive;
use std::convert::TryInto;
// use std::fmt;

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

#[derive(Enum, Clone, Copy)]
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

#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
#[allow(non_camel_case_types)]
#[repr(u32)]
enum MipsCPUOp {
    special = 0b000_000,
    j       = 0b000_010,
    jal     = 0b000_011,
    bne     = 0b000_101,
    addi    = 0b001_000,
    addiu   = 0b001_001,
    lui     = 0b001_111,
    sw      = 0b101_011,

    nop     = 0b10_000000,
    error   = 0b11_111111,
}

#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
#[allow(non_camel_case_types)]
#[repr(u32)]
enum MipsCPUFunc {
    jr    = 0b001_000,
    nop   = 0b10_000000,
    error = 0b11_111111,
}

#[derive(Debug)]
enum MipsInstructionFormat {
    Special {function: MipsCPUFunc, rs: MipsGpr, rt: MipsGpr, rd: MipsGpr},
    J {opname: MipsCPUOp, addr: u32},
    I {opname: MipsCPUOp, rs: MipsGpr, rt: MipsGpr, imm: u32},
    Error {word: u32},
}

impl fmt::Display for MipsInstructionFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MipsInstructionFormat::Special { function, rs, rt, rd} => {
                match function {
                    MipsCPUFunc::jr => {
                        assert_eq!(rt, &MipsGpr::zero);
                        assert_eq!(rd, &MipsGpr::zero);
                        write!(f, "{:?} {:?}", function, rs)
                    }
                    MipsCPUFunc::nop => {
                        write!(f, "nop")
                    }
                    _ => {
                        write!(f, "Error: unimplemented Special instruction {:?}", self)
                    }
                }
            }
            MipsInstructionFormat::J { opname, addr } => {
                match opname {
                    MipsCPUOp::j | MipsCPUOp::jal => {
                        write!(f, "{:?} {:#X}", opname, addr)
                    }
                    _ => {
                        write!(f, "Error: unimplemented J instruction {:?}", self)
                    }
                }
            }
            MipsInstructionFormat::I { opname, rs, rt, imm } => {
                match opname {
                    MipsCPUOp::bne => {
                        let signed_imm = ReallySigned(*imm as i16);

                        if rt == &MipsGpr::zero {
                            write!(f, "bnez {:?}, {:#X}", rs, signed_imm)
                        } else {
                            write!(f, "{:?} {:?}, {:?}, {:#X}", opname, rs, rt, signed_imm)
                        }
                    }
                    MipsCPUOp::lui => {
                        assert_eq!(rs, &MipsGpr::zero);
                        write!(f, "{:?} {:?}, {:#X}", opname, rt, imm)
                    }
                    MipsCPUOp::addi | MipsCPUOp::addiu => {
                        let signed_imm = ReallySigned(*imm as i16);
                        write!(f, "{:?} {:?}, {:?}, {:#X}", opname, rt, rs, signed_imm)
                    }
                    MipsCPUOp::sw => {
                        let signed_imm = ReallySigned(*imm as i16);
                        write!(f, "{:?} {:?}, {:#X}({:?})", opname, rt, signed_imm, rs)
                    }
                    _ => {
                        write!(f, "Error: unimplemented I instruction {:?}", self)
                    }
                }
            }
            MipsInstructionFormat::Error { word } => {
                write!(f, "Error: unknown instruction {:#010X}", word)
            }
        }
    }
}

fn disassemble_word(word: &u32) -> MipsInstructionFormat {
    let opcode: u32 = word >> 26;
    let opname: MipsCPUOp = opcode.try_into().unwrap();

    if word == &0 {
        // println!("nop");
        return MipsInstructionFormat::Special{function: MipsCPUFunc::nop, rs: MipsGpr::zero, rt: MipsGpr::zero, rd: MipsGpr::zero}
    }

    match opname {
        MipsCPUOp::special => {
            let funccode: u32 = word & 0x1F;
            let funcname: MipsCPUFunc = funccode.try_into().unwrap();

            match funcname {
                MipsCPUFunc::jr => {
                    let rs: MipsGpr = ((word >> 21)).try_into().unwrap();

                    assert_eq!(word & 0b000000_00000_111111111111111_000000, 0);
                    // println!("{:?} {:?}", funcname, rs);
                    MipsInstructionFormat::Special{function: funcname, rs: rs, rt: MipsGpr::zero, rd: MipsGpr::zero }
                }
                _ => {
                    println!("Unsupported function code {:#04X}", funccode);
                    MipsInstructionFormat::Error{word: *word}
                }
            }
        }
        MipsCPUOp::j | MipsCPUOp::jal => {
            let mut addr = word & 0x3FFFFFF;

            addr <<= 2;
            // println!("{:?} {:#X}", opname, addr);
            MipsInstructionFormat::J{opname: opname, addr: addr}
        }
        MipsCPUOp::bne => {
            let rs = ((word >> 21) & 0x1F).try_into().unwrap();
            let rt = ((word >> 16) & 0x1F).try_into().unwrap();
            let offset = word & 0xFFFF;

            // if rt == MipsGpr::zero {
            //     println!("bnez {:?}, {:#X}", rs, offset);
            // } else {
            //     println!("{:?} {:?}, {:?}, {:#X}", opname, rs, rt, offset);
            // }
            MipsInstructionFormat::I {opname: opname, rs: rs, rt: rt, imm: offset}
        }
        MipsCPUOp::lui => {
            let rs: MipsGpr = ((word >> 21) & 0x1F).try_into().unwrap();
            let rt = ((word >> 16) & 0x1F).try_into().unwrap();
            let imm = word & 0xFFFF;

            assert_eq!(rs, MipsGpr::zero);
            // println!("{:?} {:?}, {:#X}", opname, rt, imm);
            MipsInstructionFormat::I {opname: opname, rs: MipsGpr::zero, rt: rt, imm: imm}
        }
        MipsCPUOp::addi | MipsCPUOp::addiu => {
            let rs = ((word >> 21) & 0x1F).try_into().unwrap();
            let rt = ((word >> 16) & 0x1F).try_into().unwrap();
            let imm = word & 0xFFFF;

            // println!("{:?} {:?}, {:?}, {:#X}", opname, rt, rs, imm);
            MipsInstructionFormat::I {opname: opname, rs: rs, rt: rt, imm: imm}
        }
        MipsCPUOp::sw => {
            let base = ((word >> 21) & 0x1F).try_into().unwrap();
            let rt = ((word >> 16) & 0x1F).try_into().unwrap();
            let imm = word & 0xFFFF;

            // println!("{:?} {:?}, {:#X}({:?})", opname, rt, imm, base);
            MipsInstructionFormat::I {opname: opname, rs: base, rt: rt, imm: imm}
        }
        _ => {
            println!("Unsupported opcode {:#04X}", opcode);
            MipsInstructionFormat::Error{word: *word}
        }
    }
}


fn is_branch(instr: &MipsInstructionFormat) -> bool {
    const BRANCH_OPS: &[MipsCPUOp] = &[ MipsCPUOp::bne ];
    const BRANCH_FUNCS: &[MipsCPUFunc] = &[ MipsCPUFunc::jr ];

    match instr {
        MipsInstructionFormat::I { opname, rs: _, rt: _, imm: _} => {
            BRANCH_OPS.contains(&opname)
        }
        MipsInstructionFormat::Special { function, rs: _, rt: _, rd: _ } => {
            BRANCH_FUNCS.contains(&function)
        }
        MipsInstructionFormat::J { opname: _, addr: _ } => true,
        _ => false
    }
}

// const w: [u32; 1] = [ 0x0C000001 ];
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
    
    for word in DATA {
        let instr;

        consecutive_nops = if word == &0 { consecutive_nops + 1 } else { 0 };
        if consecutive_nops > 1 {
            println!("Second nop found, breaking out of loop.");
            break;
        }

        instr = disassemble_word(word);

        println!("{}", instr);
        if is_branch(&instr) {
            print!(" ");
        }
        match instr {
            MipsInstructionFormat::I { opname, rs, rt, imm } => {
                match opname {
                    MipsCPUOp::lui => {
                        reg_tracker[rt] = imm << 16;
                        println!("{:?} = {:#X}", rt, reg_tracker[rt])
                    }
                    MipsCPUOp::addiu => {
                        reg_tracker[rt] = reg_tracker[rs] + imm + ((imm & 0x8000) << 1)
                    }
                    MipsCPUOp::addi => {
                        if imm >= 0x8000 { bss_sign -= 1 } else { bss_sign += 1 }
                    }
                    MipsCPUOp::bne => branch_reg = rs,
                    MipsCPUOp::sw => bss_ptr_reg = rs,
                    _ => ()
                }
            }
            MipsInstructionFormat::Special { function, rs, rt: _, rd: _ } => {
                match function {
                    MipsCPUFunc::jr => jump_reg = rs,
                    _ => ()
                }
            }
            _ => ()
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
