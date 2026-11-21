//! Debugger session: a CPU with a loaded demo, step/run/reset, and the
//! disassembly window. The shell only maps keys onto these calls.

use crate::cpu::Cpu;
use crate::demos::{DEMOS, LOAD_ADDR};
use crate::disasm::disassemble;

/// Instructions executed per "run" key press, so a runaway program can't
/// freeze the UI.
pub const RUN_BUDGET: usize = 10_000;

pub struct Session {
    pub cpu: Cpu,
    pub demo: usize,
}

impl Session {
    pub fn new(demo: usize) -> Self {
        let mut s = Self { cpu: Cpu::new(), demo: demo.min(DEMOS.len() - 1) };
        s.reset();
        s
    }

    /// Reload the current demo into a fresh CPU.
    pub fn reset(&mut self) {
        self.cpu = Cpu::new();
        self.cpu.load(LOAD_ADDR, DEMOS[self.demo].code);
    }

    pub fn step(&mut self) {
        if !self.cpu.is_stopped() {
            self.cpu.step();
        }
    }

    /// Run up to [`RUN_BUDGET`] instructions or until the CPU stops. Returns
    /// how many ran.
    pub fn run(&mut self) -> usize {
        let mut n = 0;
        while n < RUN_BUDGET && !self.cpu.is_stopped() {
            self.cpu.step();
            n += 1;
        }
        n
    }

    /// One-line status: running, halted, or the fault.
    pub fn status(&self) -> String {
        if let Some(crate::cpu::Fault::IllegalOpcode { op, at }) = self.cpu.fault {
            format!("FAULT: illegal opcode ${op:02X} at ${at:04X}")
        } else if self.cpu.is_stopped() {
            "HALTED".into()
        } else if self.cpu.halted {
            "HALT (waiting for interrupt)".into()
        } else {
            "READY".into()
        }
    }

    /// `rows` instructions starting at PC: (address, text).
    pub fn listing(&self, rows: usize) -> Vec<(u16, String)> {
        let mut addr = self.cpu.regs.pc;
        let mut out = Vec::with_capacity(rows);
        for _ in 0..rows {
            let (text, len) = disassemble(&self.cpu.mem, addr);
            out.push((addr, text));
            addr = addr.wrapping_add(len as u16);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_finishes_every_demo_within_budget() {
        for i in 0..DEMOS.len() {
            let mut s = Session::new(i);
            let n = s.run();
            assert!(n < RUN_BUDGET, "{}", DEMOS[i].name);
            assert_eq!(s.status(), "HALTED");
        }
    }

    #[test]
    fn runaway_program_is_cut_off_by_the_budget() {
        let mut s = Session::new(0);
        s.cpu.load(LOAD_ADDR, &[0x18, 0xFE]); // JR self
        assert_eq!(s.run(), RUN_BUDGET);
        assert_eq!(s.status(), "READY");
    }

    #[test]
    fn step_does_nothing_once_halted_and_reset_restores() {
        let mut s = Session::new(1);
        s.run();
        let pc = s.cpu.regs.pc;
        s.step();
        assert_eq!(s.cpu.regs.pc, pc);
        s.reset();
        assert_eq!(s.cpu.regs.pc, LOAD_ADDR);
        assert_eq!(s.cpu.regs.a, 0x01);
    }

    #[test]
    fn fault_is_reported() {
        let mut s = Session::new(0);
        s.cpu.load(LOAD_ADDR, &[0xDD]);
        s.run();
        assert_eq!(s.status(), "FAULT: illegal opcode $DD at $0100");
    }

    #[test]
    fn listing_starts_at_pc_and_wraps() {
        let mut s = Session::new(1);
        let l = s.listing(3);
        assert_eq!(l[0], (0x0100, "LD A,$19".to_string()));
        assert_eq!(l[1].0, 0x0102);
        s.cpu.regs.pc = 0xFFFF;
        assert_eq!(s.listing(2).len(), 2);
    }
}
