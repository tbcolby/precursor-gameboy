//! UI rendering for Game Boy Emulator.

extern crate alloc;
use alloc::format;
use alloc::string::String;

use gam::{Gam, Point, Line, TextView, TextBounds, GlyphStyle};
use crate::app::{GbApp, AppState};
use crate::cpu::DEMOS;

const SCREEN_W: i16 = 336;
const HEADER_H: i16 = 30;
const FOOTER_Y: i16 = 490;

fn draw_header(gam: &Gam, title: &str) {
    gam.draw_rectangle(Point::new(0, 0), Point::new(SCREEN_W, HEADER_H)).ok();
    let mut tv = TextView::new(TextBounds::BoundingBox(Point::new(8, 2), Point::new(SCREEN_W - 8, HEADER_H)));
    tv.style = GlyphStyle::Bold; tv.invert = true;
    tv.text = String::from(title);
    gam.post_textview(&mut tv).ok();
}

fn draw_footer(gam: &Gam, text: &str) {
    gam.draw_line(Line::new(Point::new(0, FOOTER_Y), Point::new(SCREEN_W, FOOTER_Y))).ok();
    let mut tv = TextView::new(TextBounds::BoundingBox(Point::new(8, FOOTER_Y + 4), Point::new(SCREEN_W - 8, FOOTER_Y + 40)));
    tv.style = GlyphStyle::Small;
    tv.text = String::from(text);
    gam.post_textview(&mut tv).ok();
}

fn text_at(gam: &Gam, x: i16, y: i16, h: i16, s: &str, style: GlyphStyle) {
    let mut tv = TextView::new(TextBounds::BoundingBox(Point::new(x, y), Point::new(SCREEN_W - 4, y + h)));
    tv.style = style; tv.text = String::from(s);
    gam.post_textview(&mut tv).ok();
}

pub fn draw(gam: &Gam, app: &GbApp) {
    gam.clear().ok();
    match app.state {
        AppState::DemoSelect => draw_select(gam, app),
        AppState::Running => draw_running(gam, app),
        AppState::Memory => draw_memory(gam, app),
    }
    gam.redraw().ok();
}

fn draw_select(gam: &Gam, app: &GbApp) {
    draw_header(gam, "Game Boy Emulator");

    let start_y: i16 = 42;
    let line_h: i16 = 58;

    for (i, demo) in DEMOS.iter().enumerate() {
        let y = start_y + (i as i16) * line_h;
        if y > FOOTER_Y - line_h { break; }
        let selected = i == app.demo_cursor;

        if selected {
            gam.draw_rectangle(Point::new(4, y - 2), Point::new(SCREEN_W - 4, y + line_h - 6)).ok();
        }

        let mut tv = TextView::new(TextBounds::BoundingBox(Point::new(12, y), Point::new(SCREEN_W - 12, y + 22)));
        tv.style = GlyphStyle::Regular;
        if selected { tv.invert = true; }
        tv.text = String::from(demo.name);
        gam.post_textview(&mut tv).ok();

        let mut tv2 = TextView::new(TextBounds::BoundingBox(Point::new(16, y + 22), Point::new(SCREEN_W - 12, y + line_h)));
        tv2.style = GlyphStyle::Small;
        if selected { tv2.invert = true; }
        tv2.text = String::from(demo.description);
        gam.post_textview(&mut tv2).ok();
    }

    draw_footer(gam, "Up/Down: browse  Enter: load  Menu: quit");
}

fn draw_running(gam: &Gam, app: &GbApp) {
    let status = if app.cpu.halted { "HALTED" } else { "RUNNING" };
    draw_header(gam, &format!("CPU: {} | Inst:{}", status, app.cpu.instructions_executed));

    let mut y: i16 = 36;

    // Registers
    text_at(gam, 8, y, 16, "Registers:", GlyphStyle::Bold);
    y += 18;
    text_at(gam, 8, y, 16, &format!("A={:02X} F={:02X} BC={:04X}", app.cpu.regs.a, app.cpu.regs.f, app.cpu.regs.bc()), GlyphStyle::Small);
    y += 16;
    text_at(gam, 8, y, 16, &format!("DE={:04X} HL={:04X} SP={:04X}", app.cpu.regs.de(), app.cpu.regs.hl(), app.cpu.regs.sp), GlyphStyle::Small);
    y += 16;

    // Flags
    let flags = format!("Flags: {}{}{}{}",
        if app.cpu.regs.flag_z() { "Z" } else { "-" },
        if app.cpu.regs.flag_n() { "N" } else { "-" },
        if app.cpu.regs.flag_h() { "H" } else { "-" },
        if app.cpu.regs.flag_c() { "C" } else { "-" },
    );
    text_at(gam, 8, y, 16, &flags, GlyphStyle::Small);
    y += 16;
    text_at(gam, 8, y, 16, &format!("PC={:04X} Cycles={}", app.cpu.regs.pc, app.cpu.cycles), GlyphStyle::Small);
    y += 20;

    // Separator
    gam.draw_line(Line::new(Point::new(8, y), Point::new(SCREEN_W - 8, y))).ok();
    y += 6;

    // Disassembly
    text_at(gam, 8, y, 16, "Disassembly:", GlyphStyle::Bold);
    y += 18;

    for (i, (addr, text)) in app.disasm_lines.iter().enumerate() {
        if y > FOOTER_Y - 20 { break; }
        let is_pc = *addr == app.cpu.regs.pc;
        let marker = if is_pc { ">" } else { " " };
        let line = format!("{} {:04X}: {}", marker, addr, text);

        if is_pc {
            gam.draw_rectangle(Point::new(4, y - 1), Point::new(SCREEN_W - 4, y + 15)).ok();
            let mut tv = TextView::new(TextBounds::BoundingBox(Point::new(8, y), Point::new(SCREEN_W - 4, y + 16)));
            tv.style = GlyphStyle::Small; tv.invert = true;
            tv.text = line;
            gam.post_textview(&mut tv).ok();
        } else {
            text_at(gam, 8, y, 16, &line, GlyphStyle::Small);
        }
        y += 16;
    }

    draw_footer(gam, "Enter: step  Space: run  R: reset  M: memory  Menu: back");
}

fn draw_memory(gam: &Gam, app: &GbApp) {
    draw_header(gam, &format!("Memory @ ${:04X}", app.mem_addr));

    let mut y: i16 = 36;
    // Column header
    text_at(gam, 8, y, 16, "Addr  00 01 02 03 04 05 06 07", GlyphStyle::Bold);
    y += 18;
    gam.draw_line(Line::new(Point::new(8, y), Point::new(SCREEN_W - 8, y))).ok();
    y += 4;

    // 16 rows of 8 bytes
    for row in 0..24 {
        if y > FOOTER_Y - 16 { break; }
        let addr = app.mem_addr.wrapping_add((row * 8) as u16);
        let mut line = format!("{:04X} ", addr);
        for col in 0..8 {
            let val = app.cpu.mem.read(addr.wrapping_add(col));
            line.push_str(&format!(" {:02X}", val));
        }
        text_at(gam, 4, y, 16, &line, GlyphStyle::Small);
        y += 17;
    }

    draw_footer(gam, "Up/Down: +/-16  L/R: +/-256  Menu: back");
}
