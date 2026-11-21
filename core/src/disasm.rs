//! Disassembler. Lengths and operand layout mirror the executor's decode
//! (a test checks every opcode's length against what the CPU actually consumes).

use crate::cpu::Memory;

const R8: [&str; 8] = ["B", "C", "D", "E", "H", "L", "(HL)", "A"];
const RP: [&str; 4] = ["BC", "DE", "HL", "SP"];
const RP2: [&str; 4] = ["BC", "DE", "HL", "AF"];
const CC: [&str; 4] = ["NZ", "Z", "NC", "C"];
const ALU: [&str; 8] = ["ADD A,", "ADC A,", "SUB ", "SBC A,", "AND ", "XOR ", "OR ", "CP "];
const ROT: [&str; 8] = ["RLC", "RRC", "RL", "RR", "SLA", "SRA", "SWAP", "SRL"];

/// Disassemble the instruction at `addr`. Returns the text and its length in bytes (1-3).
pub fn disassemble(mem: &Memory, addr: u16) -> (String, u8) {
    let op = mem.read(addr);
    let n8 = || mem.read(addr.wrapping_add(1));
    let n16 = || mem.read16(addr.wrapping_add(1));
    let e8 = || {
        // Show the absolute target: pc after the instruction + offset.
        let e = n8() as i8;
        addr.wrapping_add(2).wrapping_add(e as i16 as u16)
    };
    let (x, y, z) = ((op >> 6) as usize, ((op >> 3) & 7) as usize, (op & 7) as usize);
    let (p, q) = (y >> 1, y & 1);
    match x {
        0 => match z {
            0 => match y {
                0 => ("NOP".into(), 1),
                1 => (format!("LD (${:04X}),SP", n16()), 3),
                2 => ("STOP".into(), 2),
                3 => (format!("JR ${:04X}", e8()), 2),
                _ => (format!("JR {},${:04X}", CC[y - 4], e8()), 2),
            },
            1 if q == 0 => (format!("LD {},${:04X}", RP[p], n16()), 3),
            1 => (format!("ADD HL,{}", RP[p]), 1),
            2 => {
                let m = ["(BC)", "(DE)", "(HL+)", "(HL-)"][p];
                if q == 0 { (format!("LD {m},A"), 1) } else { (format!("LD A,{m}"), 1) }
            }
            3 => (format!("{} {}", if q == 0 { "INC" } else { "DEC" }, RP[p]), 1),
            4 => (format!("INC {}", R8[y]), 1),
            5 => (format!("DEC {}", R8[y]), 1),
            6 => (format!("LD {},${:02X}", R8[y], n8()), 2),
            _ => (["RLCA", "RRCA", "RLA", "RRA", "DAA", "CPL", "SCF", "CCF"][y].into(), 1),
        },
        1 if y == 6 && z == 6 => ("HALT".into(), 1),
        1 => (format!("LD {},{}", R8[y], R8[z]), 1),
        2 => (format!("{}{}", ALU[y], R8[z]), 1),
        _ => match z {
            0 => match y {
                0..=3 => (format!("RET {}", CC[y]), 1),
                4 => (format!("LDH (${:02X}),A", n8()), 2),
                5 => (format!("ADD SP,{}", n8() as i8), 2),
                6 => (format!("LDH A,(${:02X})", n8()), 2),
                _ => (format!("LD HL,SP{:+}", n8() as i8), 2),
            },
            1 if q == 0 => (format!("POP {}", RP2[p]), 1),
            1 => (["RET", "RETI", "JP HL", "LD SP,HL"][p].into(), 1),
            2 => match y {
                0..=3 => (format!("JP {},${:04X}", CC[y], n16()), 3),
                4 => ("LD (C),A".into(), 1),
                5 => (format!("LD (${:04X}),A", n16()), 3),
                6 => ("LD A,(C)".into(), 1),
                _ => (format!("LD A,(${:04X})", n16()), 3),
            },
            3 => match y {
                0 => (format!("JP ${:04X}", n16()), 3),
                1 => {
                    let cb = n8();
                    let (cx, cy, cz) = ((cb >> 6) as usize, ((cb >> 3) & 7) as usize, (cb & 7) as usize);
                    let t = match cx {
                        0 => format!("{} {}", ROT[cy], R8[cz]),
                        1 => format!("BIT {cy},{}", R8[cz]),
                        2 => format!("RES {cy},{}", R8[cz]),
                        _ => format!("SET {cy},{}", R8[cz]),
                    };
                    (t, 2)
                }
                6 => ("DI".into(), 1),
                7 => ("EI".into(), 1),
                _ => (format!("DB ${op:02X}"), 1),
            },
            4 if y < 4 => (format!("CALL {},${:04X}", CC[y], n16()), 3),
            5 if q == 0 => (format!("PUSH {}", RP2[p]), 1),
            5 if p == 0 => (format!("CALL ${:04X}", n16()), 3),
            6 => (format!("{}${:02X}", ALU[y], n8()), 2),
            7 => (format!("RST ${:02X}", y * 8), 1),
            _ => (format!("DB ${op:02X}"), 1),
        },
    }
}
