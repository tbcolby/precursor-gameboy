//! LR35902 executor. Decode follows the regular opcode layout
//! (x = op>>6, y = bits 5..3, z = bits 2..0).
//!
//! Not modelled: the HALT bug, STOP's clock behaviour (STOP just halts),
//! memory-mapped hardware (the 64 KB is plain RAM except that 0000-7FFF is
//! read-only to program writes, like a ROM), and OAM-DMA/echo RAM.

/// Interrupt flag register address.
pub const IF_ADDR: u16 = 0xFF0F;
/// Interrupt enable register address.
pub const IE_ADDR: u16 = 0xFFFF;

pub const FLAG_Z: u8 = 0x80;
pub const FLAG_N: u8 = 0x40;
pub const FLAG_H: u8 = 0x20;
pub const FLAG_C: u8 = 0x10;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Registers {
    pub a: u8,
    pub f: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub sp: u16,
    pub pc: u16,
}

impl Default for Registers {
    fn default() -> Self { Self::new() }
}

impl Registers {
    /// Post-boot-ROM register values (DMG).
    pub fn new() -> Self {
        Self { a: 0x01, f: 0xB0, b: 0x00, c: 0x13, d: 0x00, e: 0xD8, h: 0x01, l: 0x4D, sp: 0xFFFE, pc: 0x0100 }
    }

    pub fn af(&self) -> u16 { u16::from_be_bytes([self.a, self.f]) }
    pub fn bc(&self) -> u16 { u16::from_be_bytes([self.b, self.c]) }
    pub fn de(&self) -> u16 { u16::from_be_bytes([self.d, self.e]) }
    pub fn hl(&self) -> u16 { u16::from_be_bytes([self.h, self.l]) }

    /// The low nibble of F is always zero.
    pub fn set_af(&mut self, v: u16) { self.a = (v >> 8) as u8; self.f = (v & 0xF0) as u8; }
    pub fn set_bc(&mut self, v: u16) { self.b = (v >> 8) as u8; self.c = v as u8; }
    pub fn set_de(&mut self, v: u16) { self.d = (v >> 8) as u8; self.e = v as u8; }
    pub fn set_hl(&mut self, v: u16) { self.h = (v >> 8) as u8; self.l = v as u8; }

    pub fn flag_z(&self) -> bool { self.f & FLAG_Z != 0 }
    pub fn flag_n(&self) -> bool { self.f & FLAG_N != 0 }
    pub fn flag_h(&self) -> bool { self.f & FLAG_H != 0 }
    pub fn flag_c(&self) -> bool { self.f & FLAG_C != 0 }

    fn set_flags(&mut self, z: bool, n: bool, h: bool, c: bool) {
        self.f = (if z { FLAG_Z } else { 0 }) | (if n { FLAG_N } else { 0 }) | (if h { FLAG_H } else { 0 }) | (if c { FLAG_C } else { 0 });
    }
}

/// 64 KB address space. Heap allocated.
pub struct Memory {
    pub data: Vec<u8>,
}

impl Default for Memory {
    fn default() -> Self { Self::new() }
}

impl Memory {
    pub fn new() -> Self { Self { data: vec![0u8; 0x10000] } }

    pub fn read(&self, addr: u16) -> u8 { self.data[addr as usize] }

    /// Program writes below $8000 are ignored (ROM). Use [`Memory::poke`] to bypass.
    pub fn write(&mut self, addr: u16, val: u8) {
        if addr >= 0x8000 {
            self.data[addr as usize] = val;
        }
    }

    /// Write regardless of the ROM region (loader, tests, debugger).
    pub fn poke(&mut self, addr: u16, val: u8) { self.data[addr as usize] = val; }

    pub fn read16(&self, addr: u16) -> u16 {
        u16::from_le_bytes([self.read(addr), self.read(addr.wrapping_add(1))])
    }

    pub fn write16(&mut self, addr: u16, val: u16) {
        self.write(addr, val as u8);
        self.write(addr.wrapping_add(1), (val >> 8) as u8);
    }
}

/// Why the CPU stopped executing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    /// One of the 11 opcodes the LR35902 doesn't define (it locks up).
    IllegalOpcode { op: u8, at: u16 },
}

pub struct Cpu {
    pub regs: Registers,
    pub mem: Memory,
    pub halted: bool,
    /// Interrupt master enable.
    pub ime: bool,
    /// EI takes effect after the *next* instruction.
    ei_pending: bool,
    pub fault: Option<Fault>,
    pub cycles: u64,
    pub instructions_executed: u64,
    /// Bytes consumed by the last executed instruction (opcode + operands).
    pub last_len: u8,
}

impl Default for Cpu {
    fn default() -> Self { Self::new() }
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            regs: Registers::new(),
            mem: Memory::new(),
            halted: false,
            ime: false,
            ei_pending: false,
            fault: None,
            cycles: 0,
            instructions_executed: 0,
            last_len: 0,
        }
    }

    /// Copy `data` into memory at `addr` (bypasses ROM protection; stops at $FFFF).
    pub fn load(&mut self, addr: u16, data: &[u8]) {
        for (i, &b) in data.iter().enumerate() {
            let a = addr as usize + i;
            if a >= self.mem.data.len() {
                break;
            }
            self.mem.data[a] = b;
        }
    }

    /// True when execution can't continue without outside help: faulted, or
    /// halted with no interrupt that could wake it.
    pub fn is_stopped(&self) -> bool {
        self.fault.is_some() || (self.halted && self.pending() == 0)
    }

    fn pending(&self) -> u8 { self.mem.read(IF_ADDR) & self.mem.read(IE_ADDR) & 0x1F }

    fn fetch(&mut self) -> u8 {
        let v = self.mem.read(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(1);
        self.last_len = self.last_len.wrapping_add(1);
        v
    }

    fn fetch16(&mut self) -> u16 {
        let lo = self.fetch();
        let hi = self.fetch();
        u16::from_le_bytes([lo, hi])
    }

    fn push(&mut self, v: u16) {
        self.regs.sp = self.regs.sp.wrapping_sub(2);
        self.mem.write16(self.regs.sp, v);
    }

    fn pop(&mut self) -> u16 {
        let v = self.mem.read16(self.regs.sp);
        self.regs.sp = self.regs.sp.wrapping_add(2);
        v
    }

    /// Execute one instruction (or interrupt dispatch, or one idle HALT
    /// step). Returns the cycles consumed; 0 once faulted.
    pub fn step(&mut self) -> u8 {
        if self.fault.is_some() {
            return 0;
        }
        let pending = self.pending();
        if pending != 0 {
            self.halted = false;
            if self.ime {
                let bit = pending.trailing_zeros() as u16;
                self.ime = false;
                self.ei_pending = false;
                let ifr = self.mem.read(IF_ADDR);
                self.mem.write(IF_ADDR, ifr & !(1 << bit));
                let pc = self.regs.pc;
                self.push(pc);
                self.regs.pc = 0x40 + 8 * bit;
                self.cycles += 20;
                return 20;
            }
        }
        if self.halted {
            self.cycles += 4;
            return 4;
        }
        let enable_ime = std::mem::take(&mut self.ei_pending);
        self.last_len = 0;
        let at = self.regs.pc;
        let op = self.fetch();
        self.instructions_executed += 1;
        let cycles = match self.execute(op) {
            Some(c) => c,
            None => {
                self.fault = Some(Fault::IllegalOpcode { op, at });
                self.regs.pc = at;
                return 0;
            }
        };
        if enable_ime && op != 0xF3 {
            // (EI followed directly by DI never enables.)
            self.ime = true;
        }
        self.cycles += cycles as u64;
        cycles
    }

    fn get_r(&self, i: u8) -> u8 {
        match i {
            0 => self.regs.b,
            1 => self.regs.c,
            2 => self.regs.d,
            3 => self.regs.e,
            4 => self.regs.h,
            5 => self.regs.l,
            6 => self.mem.read(self.regs.hl()),
            _ => self.regs.a,
        }
    }

    fn set_r(&mut self, i: u8, v: u8) {
        match i {
            0 => self.regs.b = v,
            1 => self.regs.c = v,
            2 => self.regs.d = v,
            3 => self.regs.e = v,
            4 => self.regs.h = v,
            5 => self.regs.l = v,
            6 => {
                let hl = self.regs.hl();
                self.mem.write(hl, v)
            }
            _ => self.regs.a = v,
        }
    }

    fn get_rp(&self, p: u8) -> u16 {
        match p {
            0 => self.regs.bc(),
            1 => self.regs.de(),
            2 => self.regs.hl(),
            _ => self.regs.sp,
        }
    }

    fn set_rp(&mut self, p: u8, v: u16) {
        match p {
            0 => self.regs.set_bc(v),
            1 => self.regs.set_de(v),
            2 => self.regs.set_hl(v),
            _ => self.regs.sp = v,
        }
    }

    fn cond(&self, y: u8) -> bool {
        match y & 3 {
            0 => !self.regs.flag_z(),
            1 => self.regs.flag_z(),
            2 => !self.regs.flag_c(),
            _ => self.regs.flag_c(),
        }
    }

    fn jr(&mut self, e: i8) { self.regs.pc = self.regs.pc.wrapping_add(e as i16 as u16); }

    /// Returns None for the 11 undefined opcodes.
    fn execute(&mut self, op: u8) -> Option<u8> {
        let (x, y, z) = (op >> 6, (op >> 3) & 7, op & 7);
        let (p, q) = (y >> 1, y & 1);
        let hl_op = |r: u8| r == 6;
        Some(match x {
            0 => match z {
                0 => match y {
                    0 => 4,
                    1 => {
                        let a = self.fetch16();
                        let sp = self.regs.sp;
                        self.mem.write16(a, sp);
                        20
                    }
                    2 => {
                        self.fetch(); // STOP is two bytes
                        self.halted = true;
                        4
                    }
                    3 => {
                        let e = self.fetch() as i8;
                        self.jr(e);
                        12
                    }
                    _ => {
                        let e = self.fetch() as i8;
                        if self.cond(y - 4) {
                            self.jr(e);
                            12
                        } else {
                            8
                        }
                    }
                },
                1 => {
                    if q == 0 {
                        let v = self.fetch16();
                        self.set_rp(p, v);
                        12
                    } else {
                        let (hl, v) = (self.regs.hl(), self.get_rp(p));
                        let r = hl as u32 + v as u32;
                        let z_ = self.regs.flag_z();
                        self.regs.set_flags(z_, false, (hl & 0xFFF) + (v & 0xFFF) > 0xFFF, r > 0xFFFF);
                        self.regs.set_hl(r as u16);
                        8
                    }
                }
                2 => {
                    let addr = match p {
                        0 => self.regs.bc(),
                        1 => self.regs.de(),
                        _ => self.regs.hl(),
                    };
                    if q == 0 {
                        let a = self.regs.a;
                        self.mem.write(addr, a);
                    } else {
                        self.regs.a = self.mem.read(addr);
                    }
                    if p == 2 {
                        self.regs.set_hl(addr.wrapping_add(1));
                    } else if p == 3 {
                        // p == 3 is (HL-); the address match above used HL for p >= 2.
                        self.regs.set_hl(addr.wrapping_sub(1));
                    }
                    8
                }
                3 => {
                    let v = self.get_rp(p);
                    self.set_rp(p, if q == 0 { v.wrapping_add(1) } else { v.wrapping_sub(1) });
                    8
                }
                4 => {
                    let v = self.get_r(y);
                    let r = v.wrapping_add(1);
                    let c = self.regs.flag_c();
                    self.regs.set_flags(r == 0, false, (v & 0xF) == 0xF, c);
                    self.set_r(y, r);
                    if hl_op(y) { 12 } else { 4 }
                }
                5 => {
                    let v = self.get_r(y);
                    let r = v.wrapping_sub(1);
                    let c = self.regs.flag_c();
                    self.regs.set_flags(r == 0, true, (v & 0xF) == 0, c);
                    self.set_r(y, r);
                    if hl_op(y) { 12 } else { 4 }
                }
                6 => {
                    let n = self.fetch();
                    self.set_r(y, n);
                    if hl_op(y) { 12 } else { 8 }
                }
                _ => {
                    let a = self.regs.a;
                    match y {
                        0 => {
                            let r = a.rotate_left(1);
                            self.regs.set_flags(false, false, false, a & 0x80 != 0);
                            self.regs.a = r;
                        }
                        1 => {
                            let r = a.rotate_right(1);
                            self.regs.set_flags(false, false, false, a & 1 != 0);
                            self.regs.a = r;
                        }
                        2 => {
                            let r = (a << 1) | self.regs.flag_c() as u8;
                            self.regs.set_flags(false, false, false, a & 0x80 != 0);
                            self.regs.a = r;
                        }
                        3 => {
                            let r = (a >> 1) | ((self.regs.flag_c() as u8) << 7);
                            self.regs.set_flags(false, false, false, a & 1 != 0);
                            self.regs.a = r;
                        }
                        4 => self.daa(),
                        5 => {
                            self.regs.a = !a;
                            self.regs.f |= FLAG_N | FLAG_H;
                        }
                        6 => {
                            self.regs.f = (self.regs.f & FLAG_Z) | FLAG_C;
                        }
                        _ => {
                            let c = !self.regs.flag_c();
                            self.regs.f = (self.regs.f & FLAG_Z) | if c { FLAG_C } else { 0 };
                        }
                    }
                    4
                }
            },
            1 => {
                if y == 6 && z == 6 {
                    self.halted = true;
                    4
                } else {
                    let v = self.get_r(z);
                    self.set_r(y, v);
                    if hl_op(y) || hl_op(z) { 8 } else { 4 }
                }
            }
            2 => {
                let v = self.get_r(z);
                self.alu(y, v);
                if hl_op(z) { 8 } else { 4 }
            }
            _ => match z {
                0 => match y {
                    0..=3 => {
                        if self.cond(y) {
                            self.regs.pc = self.pop();
                            20
                        } else {
                            8
                        }
                    }
                    4 => {
                        let n = self.fetch();
                        let a = self.regs.a;
                        self.mem.write(0xFF00 | n as u16, a);
                        12
                    }
                    5 => {
                        let e = self.fetch() as i8;
                        self.regs.sp = self.sp_plus(e);
                        16
                    }
                    6 => {
                        let n = self.fetch();
                        self.regs.a = self.mem.read(0xFF00 | n as u16);
                        12
                    }
                    _ => {
                        let e = self.fetch() as i8;
                        let v = self.sp_plus(e);
                        self.regs.set_hl(v);
                        12
                    }
                },
                1 => {
                    if q == 0 {
                        let v = self.pop();
                        match p {
                            0 => self.regs.set_bc(v),
                            1 => self.regs.set_de(v),
                            2 => self.regs.set_hl(v),
                            _ => self.regs.set_af(v),
                        }
                        12
                    } else {
                        match p {
                            0 => {
                                self.regs.pc = self.pop();
                                16
                            }
                            1 => {
                                self.regs.pc = self.pop();
                                self.ime = true;
                                16
                            }
                            2 => {
                                self.regs.pc = self.regs.hl();
                                4
                            }
                            _ => {
                                self.regs.sp = self.regs.hl();
                                8
                            }
                        }
                    }
                }
                2 => match y {
                    0..=3 => {
                        let a = self.fetch16();
                        if self.cond(y) {
                            self.regs.pc = a;
                            16
                        } else {
                            12
                        }
                    }
                    4 => {
                        let a = self.regs.a;
                        self.mem.write(0xFF00 | self.regs.c as u16, a);
                        8
                    }
                    5 => {
                        let addr = self.fetch16();
                        let a = self.regs.a;
                        self.mem.write(addr, a);
                        16
                    }
                    6 => {
                        self.regs.a = self.mem.read(0xFF00 | self.regs.c as u16);
                        8
                    }
                    _ => {
                        let addr = self.fetch16();
                        self.regs.a = self.mem.read(addr);
                        16
                    }
                },
                3 => match y {
                    0 => {
                        self.regs.pc = self.fetch16();
                        16
                    }
                    1 => {
                        let cb = self.fetch();
                        self.execute_cb(cb)
                    }
                    6 => {
                        self.ime = false;
                        self.ei_pending = false;
                        4
                    }
                    7 => {
                        self.ei_pending = true;
                        4
                    }
                    _ => return None,
                },
                4 => match y {
                    0..=3 => {
                        let a = self.fetch16();
                        if self.cond(y) {
                            let pc = self.regs.pc;
                            self.push(pc);
                            self.regs.pc = a;
                            24
                        } else {
                            12
                        }
                    }
                    _ => return None,
                },
                5 => {
                    if q == 0 {
                        let v = match p {
                            0 => self.regs.bc(),
                            1 => self.regs.de(),
                            2 => self.regs.hl(),
                            _ => self.regs.af(),
                        };
                        self.push(v);
                        16
                    } else if p == 0 {
                        let a = self.fetch16();
                        let pc = self.regs.pc;
                        self.push(pc);
                        self.regs.pc = a;
                        24
                    } else {
                        return None;
                    }
                }
                6 => {
                    let n = self.fetch();
                    self.alu(y, n);
                    8
                }
                _ => {
                    let pc = self.regs.pc;
                    self.push(pc);
                    self.regs.pc = (y as u16) * 8;
                    16
                }
            },
        })
    }

    fn sp_plus(&mut self, e: i8) -> u16 {
        let sp = self.regs.sp;
        let b = e as u8 as u16;
        self.regs.set_flags(false, false, (sp & 0xF) + (b & 0xF) > 0xF, (sp & 0xFF) + b > 0xFF);
        sp.wrapping_add(e as i16 as u16)
    }

    fn execute_cb(&mut self, op: u8) -> u8 {
        let (x, y, z) = (op >> 6, (op >> 3) & 7, op & 7);
        let v = self.get_r(z);
        let hl = z == 6;
        match x {
            0 => {
                let c_in = self.regs.flag_c() as u8;
                let (r, c) = match y {
                    0 => (v.rotate_left(1), v & 0x80 != 0),
                    1 => (v.rotate_right(1), v & 1 != 0),
                    2 => ((v << 1) | c_in, v & 0x80 != 0),
                    3 => ((v >> 1) | (c_in << 7), v & 1 != 0),
                    4 => (v << 1, v & 0x80 != 0),
                    5 => ((v >> 1) | (v & 0x80), v & 1 != 0),
                    6 => (v.rotate_left(4), false),
                    _ => (v >> 1, v & 1 != 0),
                };
                self.regs.set_flags(r == 0, false, false, c);
                self.set_r(z, r);
                if hl { 16 } else { 8 }
            }
            1 => {
                let c = self.regs.flag_c();
                self.regs.set_flags(v & (1 << y) == 0, false, true, c);
                if hl { 12 } else { 8 }
            }
            2 => {
                self.set_r(z, v & !(1 << y));
                if hl { 16 } else { 8 }
            }
            _ => {
                self.set_r(z, v | (1 << y));
                if hl { 16 } else { 8 }
            }
        }
    }

    fn alu(&mut self, op: u8, v: u8) {
        let a = self.regs.a;
        let cin = self.regs.flag_c() as u8;
        match op {
            0 | 1 => {
                let c = if op == 1 { cin } else { 0 };
                let r = a as u16 + v as u16 + c as u16;
                self.regs.set_flags(r as u8 == 0, false, (a & 0xF) + (v & 0xF) + c > 0xF, r > 0xFF);
                self.regs.a = r as u8;
            }
            2 | 3 | 7 => {
                let c = if op == 3 { cin } else { 0 };
                let r = (a as i16) - (v as i16) - (c as i16);
                self.regs.set_flags(r as u8 == 0, true, (a & 0xF) < (v & 0xF) + c, r < 0);
                if op != 7 {
                    self.regs.a = r as u8;
                }
            }
            4 => {
                self.regs.a = a & v;
                self.regs.set_flags(self.regs.a == 0, false, true, false);
            }
            5 => {
                self.regs.a = a ^ v;
                self.regs.set_flags(self.regs.a == 0, false, false, false);
            }
            _ => {
                self.regs.a = a | v;
                self.regs.set_flags(self.regs.a == 0, false, false, false);
            }
        }
    }

    fn daa(&mut self) {
        let mut a = self.regs.a;
        let (n, h) = (self.regs.flag_n(), self.regs.flag_h());
        let mut c = self.regs.flag_c();
        if !n {
            if c || a > 0x99 {
                a = a.wrapping_add(0x60);
                c = true;
            }
            if h || (a & 0x0F) > 0x09 {
                a = a.wrapping_add(0x06);
            }
        } else {
            if c {
                a = a.wrapping_sub(0x60);
            }
            if h {
                a = a.wrapping_sub(0x06);
            }
        }
        self.regs.a = a;
        self.regs.set_flags(a == 0, n, false, c);
    }
}
