//! Game Boy (Sharp LR35902) CPU core: no Xous dependencies, host-tested.
//!
//! This is a CPU and a flat 64 KB memory only: no cartridge, MBC, PPU, timer,
//! joypad or sound. It is the engine behind a step debugger, not a console.
//!
//! * [`cpu`]: all 245 opcodes plus the 256 CB-prefixed ones, interrupt
//!   dispatch, HALT wake-up
//! * [`disasm`]: disassembler built on the same decode as the executor
//! * [`demos`]: hand-assembled demo programs (each is run to HALT in tests)

pub mod cpu;
pub mod demos;
pub mod disasm;

pub use cpu::{Cpu, Fault, Memory, Registers};
