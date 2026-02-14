//! Game Boy (Sharp LR35902) CPU emulator core.
//! Implements registers, flags, memory, and instruction execution.

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

/// CPU registers.
pub struct Registers {
    pub a: u8, pub f: u8,
    pub b: u8, pub c: u8,
    pub d: u8, pub e: u8,
    pub h: u8, pub l: u8,
    pub sp: u16,
    pub pc: u16,
}

impl Registers {
    pub fn new() -> Self {
        Self {
            a: 0x01, f: 0xB0,
            b: 0x00, c: 0x13,
            d: 0x00, e: 0xD8,
            h: 0x01, l: 0x4D,
            sp: 0xFFFE,
            pc: 0x0100, // Start after boot ROM
        }
    }

    pub fn af(&self) -> u16 { ((self.a as u16) << 8) | (self.f as u16) }
    pub fn bc(&self) -> u16 { ((self.b as u16) << 8) | (self.c as u16) }
    pub fn de(&self) -> u16 { ((self.d as u16) << 8) | (self.e as u16) }
    pub fn hl(&self) -> u16 { ((self.h as u16) << 8) | (self.l as u16) }

    pub fn set_af(&mut self, v: u16) { self.a = (v >> 8) as u8; self.f = (v & 0xF0) as u8; }
    pub fn set_bc(&mut self, v: u16) { self.b = (v >> 8) as u8; self.c = v as u8; }
    pub fn set_de(&mut self, v: u16) { self.d = (v >> 8) as u8; self.e = v as u8; }
    pub fn set_hl(&mut self, v: u16) { self.h = (v >> 8) as u8; self.l = v as u8; }

    // Flag helpers
    pub fn flag_z(&self) -> bool { self.f & 0x80 != 0 }
    pub fn flag_n(&self) -> bool { self.f & 0x40 != 0 }
    pub fn flag_h(&self) -> bool { self.f & 0x20 != 0 }
    pub fn flag_c(&self) -> bool { self.f & 0x10 != 0 }

    pub fn set_flag_z(&mut self, v: bool) { if v { self.f |= 0x80; } else { self.f &= !0x80; } }
    pub fn set_flag_n(&mut self, v: bool) { if v { self.f |= 0x40; } else { self.f &= !0x40; } }
    pub fn set_flag_h(&mut self, v: bool) { if v { self.f |= 0x20; } else { self.f &= !0x20; } }
    pub fn set_flag_c(&mut self, v: bool) { if v { self.f |= 0x10; } else { self.f &= !0x10; } }
}

/// Memory (64KB address space).
pub struct Memory {
    pub data: Vec<u8>,
}

impl Memory {
    pub fn new() -> Self {
        Self { data: alloc::vec![0u8; 0x10000] }
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.data[addr as usize]
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        // Don't write to ROM area (0x0000-0x7FFF) — simplified
        if addr >= 0x8000 {
            self.data[addr as usize] = val;
        }
    }

    pub fn read16(&self, addr: u16) -> u16 {
        let lo = self.read(addr) as u16;
        let hi = self.read(addr.wrapping_add(1)) as u16;
        (hi << 8) | lo
    }

    pub fn write16(&mut self, addr: u16, val: u16) {
        self.write(addr, val as u8);
        self.write(addr.wrapping_add(1), (val >> 8) as u8);
    }
}

/// The CPU.
pub struct Cpu {
    pub regs: Registers,
    pub mem: Memory,
    pub halted: bool,
    pub cycles: u64,
    pub instructions_executed: u64,
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            regs: Registers::new(),
            mem: Memory::new(),
            halted: false,
            cycles: 0,
            instructions_executed: 0,
        }
    }

    /// Load a program into memory at a given address.
    pub fn load(&mut self, addr: u16, data: &[u8]) {
        for (i, &byte) in data.iter().enumerate() {
            let target = (addr as usize) + i;
            if target < self.mem.data.len() {
                self.mem.data[target] = byte;
            }
        }
    }

    fn fetch(&mut self) -> u8 {
        let val = self.mem.read(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(1);
        val
    }

    fn fetch16(&mut self) -> u16 {
        let lo = self.fetch() as u16;
        let hi = self.fetch() as u16;
        (hi << 8) | lo
    }

    fn push(&mut self, val: u16) {
        self.regs.sp = self.regs.sp.wrapping_sub(2);
        self.mem.write16(self.regs.sp, val);
    }

    fn pop(&mut self) -> u16 {
        let val = self.mem.read16(self.regs.sp);
        self.regs.sp = self.regs.sp.wrapping_add(2);
        val
    }

    /// Execute one instruction. Returns cycles consumed.
    pub fn step(&mut self) -> u8 {
        if self.halted { return 4; }
        let opcode = self.fetch();
        self.instructions_executed += 1;
        let cycles = self.execute(opcode);
        self.cycles += cycles as u64;
        cycles
    }

    fn execute(&mut self, op: u8) -> u8 {
        match op {
            0x00 => 4, // NOP
            0x01 => { let v = self.fetch16(); self.regs.set_bc(v); 12 } // LD BC,nn
            0x11 => { let v = self.fetch16(); self.regs.set_de(v); 12 } // LD DE,nn
            0x21 => { let v = self.fetch16(); self.regs.set_hl(v); 12 } // LD HL,nn
            0x31 => { self.regs.sp = self.fetch16(); 12 } // LD SP,nn

            0x06 => { self.regs.b = self.fetch(); 8 } // LD B,n
            0x0E => { self.regs.c = self.fetch(); 8 } // LD C,n
            0x16 => { self.regs.d = self.fetch(); 8 } // LD D,n
            0x1E => { self.regs.e = self.fetch(); 8 } // LD E,n
            0x26 => { self.regs.h = self.fetch(); 8 } // LD H,n
            0x2E => { self.regs.l = self.fetch(); 8 } // LD L,n
            0x3E => { self.regs.a = self.fetch(); 8 } // LD A,n

            0x7F => 4, // LD A,A
            0x78 => { self.regs.a = self.regs.b; 4 } // LD A,B
            0x79 => { self.regs.a = self.regs.c; 4 } // LD A,C
            0x7A => { self.regs.a = self.regs.d; 4 } // LD A,D
            0x7B => { self.regs.a = self.regs.e; 4 } // LD A,E
            0x7C => { self.regs.a = self.regs.h; 4 } // LD A,H
            0x7D => { self.regs.a = self.regs.l; 4 } // LD A,L

            // ADD A,r
            0x80 => { self.add_a(self.regs.b); 4 }
            0x81 => { self.add_a(self.regs.c); 4 }
            0x82 => { self.add_a(self.regs.d); 4 }
            0x83 => { self.add_a(self.regs.e); 4 }
            0x84 => { self.add_a(self.regs.h); 4 }
            0x85 => { self.add_a(self.regs.l); 4 }
            0xC6 => { let n = self.fetch(); self.add_a(n); 8 } // ADD A,n

            // SUB r
            0x90 => { self.sub_a(self.regs.b); 4 }
            0x91 => { self.sub_a(self.regs.c); 4 }

            // INC/DEC
            0x04 => { self.regs.b = self.inc(self.regs.b); 4 }
            0x0C => { self.regs.c = self.inc(self.regs.c); 4 }
            0x14 => { self.regs.d = self.inc(self.regs.d); 4 }
            0x1C => { self.regs.e = self.inc(self.regs.e); 4 }
            0x24 => { self.regs.h = self.inc(self.regs.h); 4 }
            0x2C => { self.regs.l = self.inc(self.regs.l); 4 }
            0x3C => { self.regs.a = self.inc(self.regs.a); 4 }

            0x05 => { self.regs.b = self.dec(self.regs.b); 4 }
            0x0D => { self.regs.c = self.dec(self.regs.c); 4 }
            0x15 => { self.regs.d = self.dec(self.regs.d); 4 }
            0x1D => { self.regs.e = self.dec(self.regs.e); 4 }
            0x25 => { self.regs.h = self.dec(self.regs.h); 4 }
            0x2D => { self.regs.l = self.dec(self.regs.l); 4 }
            0x3D => { self.regs.a = self.dec(self.regs.a); 4 }

            // AND/OR/XOR/CP
            0xA7 => { self.and_a(self.regs.a); 4 }
            0xB7 => { self.or_a(self.regs.a); 4 }
            0xAF => { self.xor_a(self.regs.a); 4 } // XOR A (clears A)
            0xFE => { let n = self.fetch(); self.cp_a(n); 8 } // CP n

            // Jumps
            0xC3 => { self.regs.pc = self.fetch16(); 16 } // JP nn
            0xC2 => { let addr = self.fetch16(); if !self.regs.flag_z() { self.regs.pc = addr; 16 } else { 12 } }
            0xCA => { let addr = self.fetch16(); if self.regs.flag_z() { self.regs.pc = addr; 16 } else { 12 } }
            0x18 => { let off = self.fetch() as i8; self.regs.pc = (self.regs.pc as i32 + off as i32) as u16; 12 } // JR n
            0x20 => { let off = self.fetch() as i8; if !self.regs.flag_z() { self.regs.pc = (self.regs.pc as i32 + off as i32) as u16; 12 } else { 8 } }
            0x28 => { let off = self.fetch() as i8; if self.regs.flag_z() { self.regs.pc = (self.regs.pc as i32 + off as i32) as u16; 12 } else { 8 } }

            // CALL/RET
            0xCD => { let addr = self.fetch16(); self.push(self.regs.pc); self.regs.pc = addr; 24 }
            0xC9 => { self.regs.pc = self.pop(); 16 }

            // PUSH/POP
            0xC5 => { let v = self.regs.bc(); self.push(v); 16 }
            0xD5 => { let v = self.regs.de(); self.push(v); 16 }
            0xE5 => { let v = self.regs.hl(); self.push(v); 16 }
            0xF5 => { let v = self.regs.af(); self.push(v); 16 }
            0xC1 => { let v = self.pop(); self.regs.set_bc(v); 12 }
            0xD1 => { let v = self.pop(); self.regs.set_de(v); 12 }
            0xE1 => { let v = self.pop(); self.regs.set_hl(v); 12 }
            0xF1 => { let v = self.pop(); self.regs.set_af(v); 12 }

            // LD (HL),A and LD A,(HL)
            0x77 => { let hl = self.regs.hl(); self.mem.write(hl, self.regs.a); 8 }
            0x7E => { self.regs.a = self.mem.read(self.regs.hl()); 8 }

            // HALT
            0x76 => { self.halted = true; 4 }

            // DI / EI (simplified — no interrupt emulation)
            0xF3 => 4, // DI
            0xFB => 4, // EI

            _ => 4, // Unknown opcode — NOP equivalent
        }
    }

    fn add_a(&mut self, val: u8) {
        let a = self.regs.a;
        let result = a as u16 + val as u16;
        self.regs.a = result as u8;
        self.regs.set_flag_z(self.regs.a == 0);
        self.regs.set_flag_n(false);
        self.regs.set_flag_h((a & 0xF) + (val & 0xF) > 0xF);
        self.regs.set_flag_c(result > 0xFF);
    }

    fn sub_a(&mut self, val: u8) {
        let a = self.regs.a;
        let result = a.wrapping_sub(val);
        self.regs.a = result;
        self.regs.set_flag_z(result == 0);
        self.regs.set_flag_n(true);
        self.regs.set_flag_h((a & 0xF) < (val & 0xF));
        self.regs.set_flag_c(a < val);
    }

    fn and_a(&mut self, val: u8) {
        self.regs.a &= val;
        self.regs.set_flag_z(self.regs.a == 0);
        self.regs.set_flag_n(false);
        self.regs.set_flag_h(true);
        self.regs.set_flag_c(false);
    }

    fn or_a(&mut self, val: u8) {
        self.regs.a |= val;
        self.regs.set_flag_z(self.regs.a == 0);
        self.regs.set_flag_n(false);
        self.regs.set_flag_h(false);
        self.regs.set_flag_c(false);
    }

    fn xor_a(&mut self, val: u8) {
        self.regs.a ^= val;
        self.regs.set_flag_z(self.regs.a == 0);
        self.regs.set_flag_n(false);
        self.regs.set_flag_h(false);
        self.regs.set_flag_c(false);
    }

    fn cp_a(&mut self, val: u8) {
        let a = self.regs.a;
        self.regs.set_flag_z(a == val);
        self.regs.set_flag_n(true);
        self.regs.set_flag_h((a & 0xF) < (val & 0xF));
        self.regs.set_flag_c(a < val);
    }

    fn inc(&mut self, val: u8) -> u8 {
        let result = val.wrapping_add(1);
        self.regs.set_flag_z(result == 0);
        self.regs.set_flag_n(false);
        self.regs.set_flag_h((val & 0xF) + 1 > 0xF);
        result
    }

    fn dec(&mut self, val: u8) -> u8 {
        let result = val.wrapping_sub(1);
        self.regs.set_flag_z(result == 0);
        self.regs.set_flag_n(true);
        self.regs.set_flag_h(val & 0xF == 0);
        result
    }
}

/// Disassemble one instruction at the given address.
pub fn disassemble(mem: &Memory, addr: u16) -> (String, u16) {
    let op = mem.read(addr);
    let next_addr;
    let text = match op {
        0x00 => { next_addr = addr + 1; String::from("NOP") }
        0x01 => { let v = mem.read16(addr + 1); next_addr = addr + 3; format!("LD BC,${:04X}", v) }
        0x11 => { let v = mem.read16(addr + 1); next_addr = addr + 3; format!("LD DE,${:04X}", v) }
        0x21 => { let v = mem.read16(addr + 1); next_addr = addr + 3; format!("LD HL,${:04X}", v) }
        0x31 => { let v = mem.read16(addr + 1); next_addr = addr + 3; format!("LD SP,${:04X}", v) }
        0x3E => { next_addr = addr + 2; format!("LD A,${:02X}", mem.read(addr + 1)) }
        0x06 => { next_addr = addr + 2; format!("LD B,${:02X}", mem.read(addr + 1)) }
        0x0E => { next_addr = addr + 2; format!("LD C,${:02X}", mem.read(addr + 1)) }
        0xC3 => { let v = mem.read16(addr + 1); next_addr = addr + 3; format!("JP ${:04X}", v) }
        0xCD => { let v = mem.read16(addr + 1); next_addr = addr + 3; format!("CALL ${:04X}", v) }
        0xC9 => { next_addr = addr + 1; String::from("RET") }
        0x76 => { next_addr = addr + 1; String::from("HALT") }
        0xAF => { next_addr = addr + 1; String::from("XOR A") }
        0x18 => { let off = mem.read(addr + 1) as i8; next_addr = addr + 2; format!("JR {}", off) }
        0x20 => { let off = mem.read(addr + 1) as i8; next_addr = addr + 2; format!("JR NZ,{}", off) }
        0x28 => { let off = mem.read(addr + 1) as i8; next_addr = addr + 2; format!("JR Z,{}", off) }
        0xFE => { next_addr = addr + 2; format!("CP ${:02X}", mem.read(addr + 1)) }
        _ => { next_addr = addr + 1; format!("DB ${:02X}", op) }
    };
    (text, next_addr)
}

/// Demo programs.
pub struct DemoProgram {
    pub name: &'static str,
    pub description: &'static str,
    pub code: &'static [u8],
}

pub static DEMOS: &[DemoProgram] = &[
    DemoProgram {
        name: "Counter",
        description: "Count from 0 to 10 in register A",
        code: &[
            0xAF,       // XOR A (A=0)
            0x3C,       // INC A
            0xFE, 0x0B, // CP $0B (11)
            0x20, 0xFC, // JR NZ,-4 (loop)
            0x76,       // HALT
        ],
    },
    DemoProgram {
        name: "Add Two Numbers",
        description: "Load 25+37 into A",
        code: &[
            0x3E, 0x19, // LD A,$19 (25)
            0x06, 0x25, // LD B,$25 (37)
            0x80,       // ADD A,B
            0x76,       // HALT
        ],
    },
    DemoProgram {
        name: "Fibonacci",
        description: "First 8 Fibonacci numbers in memory",
        code: &[
            0x21, 0x00, 0xC0, // LD HL,$C000
            0x3E, 0x01,       // LD A,$01
            0x06, 0x00,       // LD B,$00
            0x77,             // LD (HL),A (store fib)
            0x2C,             // INC L
            0x0E, 0x00,       // LD C,$00 (temp)
            0x4F,             // LD C,A
            0x80,             // ADD A,B
            0x41,             // LD B,C
            0x77,             // LD (HL),A
            0x2C,             // INC L
            0x7D,             // LD A,L
            0xFE, 0x08,       // CP $08
            0x20, 0xF6,       // JR NZ,-10
            0x76,             // HALT
        ],
    },
    DemoProgram {
        name: "Memory Fill",
        description: "Fill C000-C0FF with $42",
        code: &[
            0x21, 0x00, 0xC0, // LD HL,$C000
            0x3E, 0x42,       // LD A,$42
            0x77,             // LD (HL),A
            0x2C,             // INC L
            0x20, 0xFC,       // JR NZ,-4
            0x76,             // HALT
        ],
    },
];
