//! Screens, input and drawing for the Game Boy step debugger. All CPU
//! behaviour is in `gameboy-core`.

use gam::menu::Point;
use gam::GlyphStyle;
use gameboy_core::demos::DEMOS;
use gameboy_core::session::Session;
use precursor_appkit::keys::Key;
use precursor_appkit::layout::ListState;
use precursor_appkit::screen::Screen;

#[derive(PartialEq, Eq)]
pub enum Action {
    Stay,
    Leave,
}

enum Mode {
    Demos,
    Running,
    Memory,
}

/// Bytes per memory-viewer row.
const MEM_COLS: u16 = 8;

pub struct App {
    screen: Screen,
    mode: Mode,
    list: ListState,
    visible_rows: usize,
    session: Session,
    mem_addr: u16,
    foreground: bool,
}

impl App {
    pub fn new(screen: Screen) -> Self {
        Self {
            screen,
            mode: Mode::Demos,
            list: ListState::default(),
            visible_rows: 5,
            session: Session::new(0),
            mem_addr: 0xC000,
            foreground: true,
        }
    }

    pub fn screen(&self) -> &Screen { &self.screen }

    pub fn on_foreground(&mut self) {
        self.foreground = true;
        self.screen.refresh_size();
        self.draw();
    }

    pub fn on_background(&mut self) { self.foreground = false; }

    pub fn on_key(&mut self, key: Key) -> Action {
        match self.mode {
            Mode::Demos => match key {
                k if k.is_back() => return Action::Leave,
                Key::Up => self.list.step(-1, DEMOS.len(), self.visible_rows),
                Key::Down => self.list.step(1, DEMOS.len(), self.visible_rows),
                Key::Enter => {
                    self.session = Session::new(self.list.selected);
                    self.mode = Mode::Running;
                }
                _ => {}
            },
            Mode::Running => match key {
                k if k.is_back() => self.mode = Mode::Demos,
                Key::Enter => self.session.step(),
                Key::Char(' ') => {
                    self.session.run();
                }
                Key::Char('r') | Key::Char('R') => self.session.reset(),
                Key::Char('m') | Key::Char('M') => {
                    self.mem_addr = 0xC000;
                    self.mode = Mode::Memory;
                }
                _ => {}
            },
            Mode::Memory => match key {
                k if k.is_back() => self.mode = Mode::Running,
                Key::Down => self.mem_addr = self.mem_addr.wrapping_add(MEM_COLS),
                Key::Up => self.mem_addr = self.mem_addr.wrapping_sub(MEM_COLS),
                Key::Right => self.mem_addr = self.mem_addr.wrapping_add(0x100),
                Key::Left => self.mem_addr = self.mem_addr.wrapping_sub(0x100),
                Key::Char('p') | Key::Char('P') => self.mem_addr = self.session.cpu.regs.pc,
                Key::Char('s') | Key::Char('S') => self.mem_addr = self.session.cpu.regs.sp,
                Key::Char('c') | Key::Char('C') => self.mem_addr = 0xC000,
                _ => {}
            },
        }
        Action::Stay
    }

    pub fn draw(&mut self) {
        if !self.foreground {
            return;
        }
        self.screen.clear();
        match self.mode {
            Mode::Demos => self.draw_demos(),
            Mode::Running => self.draw_running(),
            Mode::Memory => self.draw_memory(),
        }
        self.screen.flush();
    }

    fn draw_demos(&mut self) {
        let top = self.screen.header("Game Boy CPU");
        let bottom = self.screen.footer("↑↓ select · ⏎ load · ⌫ leave");
        let labels: Vec<String> = DEMOS.iter().map(|d| format!("{}: {}", d.name, d.description)).collect();
        self.visible_rows = self.screen.list(top, bottom, &labels, &self.list);
    }

    fn draw_running(&mut self) {
        let s = &self.session;
        let r = &s.cpu.regs;
        let top = self.screen.header(&format!("{} · {}", DEMOS[s.demo].name, s.status()));
        let bottom = self.screen.footer("⏎ step · space run · r reset · m mem · ⌫ back");
        let lh = self.screen.line_height(GlyphStyle::Small);
        let flags = format!(
            "{}{}{}{}",
            if r.flag_z() { 'Z' } else { '-' },
            if r.flag_n() { 'N' } else { '-' },
            if r.flag_h() { 'H' } else { '-' },
            if r.flag_c() { 'C' } else { '-' },
        );
        let lines = [
            format!("A={:02X} F={:02X} BC={:04X} DE={:04X}", r.a, r.f, r.bc(), r.de()),
            format!("HL={:04X} SP={:04X} PC={:04X}", r.hl(), r.sp, r.pc),
            format!("{flags}  IME={}  cycles={}  instr={}", s.cpu.ime as u8, s.cpu.cycles, s.cpu.instructions_executed),
        ];
        let mut y = top;
        for l in &lines {
            self.screen.text(y, GlyphStyle::Small, l);
            y += lh;
        }
        y += 4;
        self.screen.line(Point::new(0, y), Point::new(self.screen.size().x - 1, y));
        y += 4;
        let rows = ((bottom - y) / (lh + 2)).max(1) as usize;
        for (i, (addr, text)) in s.listing(rows).iter().enumerate() {
            let marker = if i == 0 { "▸" } else { " " };
            let row_tl = Point::new(0, y);
            if i == 0 {
                // Selected-row marker as a frame: TextView invert is ignored on app canvases.
                self.screen.frame(row_tl, Point::new(self.screen.size().x - 1, y + lh + 1), 1);
            }
            self.screen.text(y + 1, GlyphStyle::Small, &format!("{marker}{addr:04X}  {text}"));
            y += lh + 2;
        }
    }

    fn draw_memory(&mut self) {
        let top = self.screen.header(&format!("Memory ${:04X}", self.mem_addr));
        let bottom = self.screen.footer("↑↓ ±8 · ←→ ±256 · p PC · s SP · c $C000 · ⌫");
        let lh = self.screen.line_height(GlyphStyle::Small) + 2;
        let rows = ((bottom - top) / lh).max(1) as usize;
        let mut y = top;
        for row in 0..rows {
            let addr = self.mem_addr.wrapping_add(row as u16 * MEM_COLS);
            let mut line = format!("{addr:04X}");
            for col in 0..MEM_COLS {
                line.push_str(&format!(" {:02X}", self.session.cpu.mem.read(addr.wrapping_add(col))));
            }
            self.screen.text(y, GlyphStyle::Small, &line);
            y += lh;
        }
    }
}
