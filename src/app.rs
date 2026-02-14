//! State machine for Game Boy Emulator.
//! States: DemoSelect, Running, Memory

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use crate::cpu::{self, Cpu, DEMOS};

const KEY_UP: char = '\u{F700}';
const KEY_DOWN: char = '\u{F701}';
const KEY_LEFT: char = '\u{F702}';
const KEY_RIGHT: char = '\u{F703}';
const KEY_ENTER: char = '\u{000D}';
const KEY_BACKSPACE: char = '\u{0008}';
const KEY_MENU: char = '\u{2234}';

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppState { DemoSelect, Running, Memory }

pub struct GbApp {
    pub state: AppState,
    pub needs_redraw: bool,
    pub cpu: Cpu,
    pub demo_cursor: usize,
    pub disasm_lines: Vec<(u16, String)>,
    pub disasm_scroll: usize,
    pub mem_addr: u16,
    pub paused: bool,
    pub step_mode: bool,
}

impl GbApp {
    pub fn new() -> Self {
        Self {
            state: AppState::DemoSelect,
            needs_redraw: true,
            cpu: Cpu::new(),
            demo_cursor: 0,
            disasm_lines: Vec::new(),
            disasm_scroll: 0,
            mem_addr: 0x0100,
            paused: true,
            step_mode: true,
        }
    }

    pub fn load_demo(&mut self, idx: usize) {
        self.cpu = Cpu::new();
        let demo = &DEMOS[idx];
        self.cpu.load(0x0100, demo.code);
        self.disassemble_around_pc();
        self.paused = true;
        self.step_mode = true;
    }

    pub fn disassemble_around_pc(&mut self) {
        self.disasm_lines.clear();
        let mut addr = self.cpu.regs.pc;
        for _ in 0..20 {
            let (text, next) = cpu::disassemble(&self.cpu.mem, addr);
            self.disasm_lines.push((addr, text));
            addr = next;
        }
    }

    pub fn step_cpu(&mut self) {
        if !self.cpu.halted {
            self.cpu.step();
            self.disassemble_around_pc();
        }
    }

    pub fn handle_key(&mut self, key: char) -> bool {
        self.needs_redraw = true;
        match self.state {
            AppState::DemoSelect => self.key_select(key),
            AppState::Running => self.key_running(key),
            AppState::Memory => self.key_memory(key),
        }
    }

    fn key_select(&mut self, key: char) -> bool {
        match key {
            KEY_MENU => return false,
            KEY_UP => { if self.demo_cursor > 0 { self.demo_cursor -= 1; } }
            KEY_DOWN => { if self.demo_cursor < DEMOS.len() - 1 { self.demo_cursor += 1; } }
            KEY_ENTER => {
                self.load_demo(self.demo_cursor);
                self.state = AppState::Running;
            }
            _ => {}
        }
        true
    }

    fn key_running(&mut self, key: char) -> bool {
        match key {
            KEY_MENU => { self.state = AppState::DemoSelect; }
            KEY_ENTER => {
                // Single step
                self.step_cpu();
            }
            ' ' => {
                // Run until halt (max 1000 steps)
                for _ in 0..1000 {
                    if self.cpu.halted { break; }
                    self.cpu.step();
                }
                self.disassemble_around_pc();
            }
            'r' | 'R' => {
                // Reset
                self.load_demo(self.demo_cursor);
            }
            'm' | 'M' => {
                self.mem_addr = 0xC000;
                self.state = AppState::Memory;
            }
            KEY_DOWN => {
                if self.disasm_scroll < self.disasm_lines.len().saturating_sub(1) { self.disasm_scroll += 1; }
            }
            KEY_UP => { if self.disasm_scroll > 0 { self.disasm_scroll -= 1; } }
            _ => {}
        }
        true
    }

    fn key_memory(&mut self, key: char) -> bool {
        match key {
            KEY_MENU | KEY_LEFT => { self.state = AppState::Running; }
            KEY_DOWN => { self.mem_addr = self.mem_addr.wrapping_add(16); }
            KEY_UP => { self.mem_addr = self.mem_addr.wrapping_sub(16); }
            KEY_RIGHT => { self.mem_addr = self.mem_addr.wrapping_add(256); }
            KEY_LEFT => { self.mem_addr = self.mem_addr.wrapping_sub(256); }
            _ => {}
        }
        true
    }
}
