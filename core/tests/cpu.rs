use gameboy_core::cpu::{Cpu, Fault, FLAG_C, FLAG_H, FLAG_N, FLAG_Z, IE_ADDR, IF_ADDR};
use gameboy_core::demos::{DEMOS, LOAD_ADDR};
use gameboy_core::disasm::disassemble;

/// A CPU with `code` at $0100.
fn cpu_with(code: &[u8]) -> Cpu {
    let mut c = Cpu::new();
    c.load(LOAD_ADDR, code);
    c
}

/// Execute `n` instructions.
fn run(c: &mut Cpu, n: usize) {
    for _ in 0..n {
        c.step();
    }
}

fn run_to_halt(c: &mut Cpu) -> u64 {
    let mut steps = 0;
    while !c.halted && c.fault.is_none() {
        c.step();
        steps += 1;
        assert!(steps < 100_000, "did not halt");
    }
    steps
}

fn demo(name: &str) -> Cpu {
    let d = DEMOS.iter().find(|d| d.name == name).unwrap();
    let mut c = cpu_with(d.code);
    run_to_halt(&mut c);
    assert!(c.fault.is_none());
    c
}

// ---- demos (audit G-3, G-4) -------------------------------------------------

#[test]
fn counter_demo_halts_with_a_eleven() {
    let c = demo("Counter");
    assert_eq!(c.regs.a, 11);
    assert!(c.regs.flag_z());
}

#[test]
fn add_demo_gives_62() {
    let c = demo("Add Two Numbers");
    assert_eq!(c.regs.a, 62);
}

#[test]
fn fibonacci_demo_writes_eight_numbers() {
    let c = demo("Fibonacci");
    let got: Vec<u8> = (0..9).map(|i| c.mem.read(0xC000 + i)).collect();
    assert_eq!(got, [1, 1, 2, 3, 5, 8, 13, 21, 0]);
}

#[test]
fn memory_fill_demo_fills_exactly_256_bytes() {
    let c = demo("Memory Fill");
    assert!((0xC000u16..=0xC0FF).all(|a| c.mem.read(a) == 0x42));
    assert_eq!(c.mem.read(0xC100), 0);
    assert_eq!(c.mem.read(0xBFFF), 0);
}

#[test]
fn bcd_demo_uses_daa() {
    let c = demo("BCD Add");
    assert_eq!(c.regs.a, 0x47);
}

#[test]
fn every_demo_halts_without_fault() {
    for d in DEMOS {
        let mut c = cpu_with(d.code);
        run_to_halt(&mut c);
        assert!(c.halted && c.fault.is_none(), "{}", d.name);
    }
}

// ---- HALT (audit: halt handling) --------------------------------------------

#[test]
fn halt_with_nothing_pending_is_stopped_and_counts_cycles() {
    let mut c = cpu_with(&[0x76, 0x00]);
    c.step();
    assert!(c.halted && c.is_stopped());
    let before = c.cycles;
    assert_eq!(c.step(), 4);
    assert_eq!(c.cycles, before + 4);
    assert_eq!(c.regs.pc, 0x0101, "PC stays after HALT");
}

#[test]
fn halt_wakes_on_pending_interrupt_without_ime() {
    let mut c = cpu_with(&[0x76, 0x3C, 0x76]); // HALT; INC A; HALT
    c.step();
    assert!(c.is_stopped());
    c.mem.poke(IE_ADDR, 0x01);
    c.mem.poke(IF_ADDR, 0x01);
    assert!(!c.is_stopped());
    c.step(); // wakes and runs INC A (IME off: no dispatch)
    assert!(!c.halted);
    assert_eq!(c.regs.a, 0x02);
}

#[test]
fn interrupt_dispatch_pushes_pc_and_clears_flag() {
    let mut c = cpu_with(&[0xFB, 0x00, 0x00]); // EI; NOP; NOP
    c.mem.poke(IE_ADDR, 0x04); // timer
    c.step(); // EI
    assert!(!c.ime, "EI takes effect after the next instruction");
    c.mem.poke(IF_ADDR, 0x04);
    c.step(); // NOP, then IME on
    assert!(c.ime);
    let cycles = c.step(); // dispatch
    assert_eq!(cycles, 20);
    assert_eq!(c.regs.pc, 0x0050);
    assert!(!c.ime);
    assert_eq!(c.mem.read(IF_ADDR) & 0x04, 0);
    assert_eq!(c.regs.sp, 0xFFFC);
    assert_eq!(c.mem.read16(0xFFFC), 0x0102);
}

#[test]
fn interrupt_priority_is_lowest_bit_first() {
    let mut c = cpu_with(&[0x00]);
    c.ime = true;
    c.mem.poke(IE_ADDR, 0x1F);
    c.mem.poke(IF_ADDR, 0x12);
    c.step();
    assert_eq!(c.regs.pc, 0x0048);
    assert_eq!(c.mem.read(IF_ADDR), 0x10);
}

#[test]
fn halt_wakes_into_handler_when_ime_set() {
    let mut c = cpu_with(&[0xFB, 0x00, 0x76, 0x00]);
    c.mem.poke(IE_ADDR, 0x01);
    run(&mut c, 3); // EI NOP HALT
    assert!(c.halted && c.ime);
    c.mem.poke(IF_ADDR, 0x01);
    c.step();
    assert!(!c.halted);
    assert_eq!(c.regs.pc, 0x0040);
    assert_eq!(c.mem.read16(c.regs.sp), 0x0103, "returns after the HALT");
}

#[test]
fn reti_returns_and_enables_interrupts() {
    let mut c = cpu_with(&[0xD9]);
    c.regs.sp = 0xC000;
    c.mem.poke(0xC000, 0x34);
    c.mem.poke(0xC001, 0x12);
    c.step();
    assert_eq!(c.regs.pc, 0x1234);
    assert!(c.ime);
}

// ---- illegal opcodes --------------------------------------------------------

#[test]
fn eleven_opcodes_fault_and_the_rest_do_not() {
    let illegal = [0xD3, 0xDB, 0xDD, 0xE3, 0xE4, 0xEB, 0xEC, 0xED, 0xF4, 0xFC, 0xFD];
    for op in 0..=255u8 {
        let mut c = cpu_with(&[op, 0, 0]);
        c.step();
        assert_eq!(c.fault.is_some(), illegal.contains(&op), "opcode {op:02X}");
    }
}

#[test]
fn fault_reports_opcode_and_address_and_sticks() {
    let mut c = cpu_with(&[0x00, 0xD3]);
    run(&mut c, 2);
    assert_eq!(c.fault, Some(Fault::IllegalOpcode { op: 0xD3, at: 0x0101 }));
    assert_eq!(c.regs.pc, 0x0101);
    assert_eq!(c.step(), 0);
    assert!(c.is_stopped());
}

// ---- 8-bit ALU and flags ----------------------------------------------------

#[test]
fn add_flags() {
    let mut c = cpu_with(&[0x3E, 0x0F, 0xC6, 0x01, 0x3E, 0xF0, 0xC6, 0x10, 0x3E, 0x3A, 0xC6, 0xC6]);
    run(&mut c, 2);
    assert_eq!((c.regs.a, c.regs.f), (0x10, FLAG_H));
    run(&mut c, 2);
    assert_eq!((c.regs.a, c.regs.f), (0x00, FLAG_Z | FLAG_C));
    run(&mut c, 2);
    assert_eq!((c.regs.a, c.regs.f), (0x00, FLAG_Z | FLAG_H | FLAG_C));
}

#[test]
fn adc_uses_carry_in_for_result_and_half_carry() {
    // SCF; LD A,$0F; ADC A,$00 -> $10, H set
    let mut c = cpu_with(&[0x37, 0x3E, 0x0F, 0xCE, 0x00]);
    run(&mut c, 3);
    assert_eq!((c.regs.a, c.regs.f), (0x10, FLAG_H));
    // SCF; LD A,$FF; ADC A,$00 -> $00 Z H C
    let mut c = cpu_with(&[0x37, 0x3E, 0xFF, 0xCE, 0x00]);
    run(&mut c, 3);
    assert_eq!((c.regs.a, c.regs.f), (0x00, FLAG_Z | FLAG_H | FLAG_C));
}

#[test]
fn sub_cp_and_sbc_flags() {
    let mut c = cpu_with(&[0x3E, 0x10, 0xD6, 0x01]); // SUB 1
    run(&mut c, 2);
    assert_eq!((c.regs.a, c.regs.f), (0x0F, FLAG_N | FLAG_H));
    let mut c = cpu_with(&[0x3E, 0x00, 0xD6, 0x01]);
    run(&mut c, 2);
    assert_eq!((c.regs.a, c.regs.f), (0xFF, FLAG_N | FLAG_H | FLAG_C));
    let mut c = cpu_with(&[0x3E, 0x05, 0xFE, 0x05]); // CP: A unchanged
    run(&mut c, 2);
    assert_eq!((c.regs.a, c.regs.f), (0x05, FLAG_Z | FLAG_N));
    // SCF; LD A,$00; SBC A,$00 -> $FF with H and C
    let mut c = cpu_with(&[0x37, 0x3E, 0x00, 0xDE, 0x00]);
    run(&mut c, 3);
    assert_eq!((c.regs.a, c.regs.f), (0xFF, FLAG_N | FLAG_H | FLAG_C));
    // SCF; LD A,$01; SBC A,$00 -> $00, Z N only
    let mut c = cpu_with(&[0x37, 0x3E, 0x01, 0xDE, 0x00]);
    run(&mut c, 3);
    assert_eq!((c.regs.a, c.regs.f), (0x00, FLAG_Z | FLAG_N));
}

#[test]
fn logic_ops_set_flags() {
    let mut c = cpu_with(&[0x3E, 0xF0, 0xE6, 0x0F]); // AND $0F
    run(&mut c, 2);
    assert_eq!((c.regs.a, c.regs.f), (0, FLAG_Z | FLAG_H));
    let mut c = cpu_with(&[0x3E, 0xF0, 0xF6, 0x0F]); // OR
    run(&mut c, 2);
    assert_eq!((c.regs.a, c.regs.f), (0xFF, 0));
    let mut c = cpu_with(&[0x3E, 0xFF, 0xEE, 0xFF]); // XOR
    run(&mut c, 2);
    assert_eq!((c.regs.a, c.regs.f), (0, FLAG_Z));
}

#[test]
fn inc_dec_preserve_carry_and_set_half() {
    let mut c = cpu_with(&[0x37, 0x06, 0x0F, 0x04, 0x05]); // SCF; LD B,$0F; INC B; DEC B
    run(&mut c, 3);
    assert_eq!((c.regs.b, c.regs.f), (0x10, FLAG_H | FLAG_C));
    c.step();
    assert_eq!((c.regs.b, c.regs.f), (0x0F, FLAG_N | FLAG_H | FLAG_C));
    let mut c = cpu_with(&[0x06, 0xFF, 0x04]);
    run(&mut c, 2);
    assert_eq!((c.regs.b, c.regs.f), (0, FLAG_Z | FLAG_H | FLAG_C) /* reset F has C set; INC keeps it */);
}

#[test]
fn inc_hl_indirect_touches_memory() {
    let mut c = cpu_with(&[0x21, 0x00, 0xC0, 0x34, 0x35, 0x35]); // INC (HL); DEC (HL) x2
    run(&mut c, 2);
    assert_eq!(c.mem.read(0xC000), 1);
    run(&mut c, 2);
    assert_eq!(c.mem.read(0xC000), 0xFF);
}

#[test]
fn ld_r_r_block_and_hl_forms() {
    let mut c = cpu_with(&[0x06, 0x12, 0x48, 0x51, 0x6A, 0x21, 0x00, 0xC0, 0x70, 0x5E]);
    // LD B,$12; LD C,B; LD D,C; LD L,D; LD HL,$C000; LD (HL),B; LD E,(HL)
    run(&mut c, 4);
    assert_eq!((c.regs.c, c.regs.d, c.regs.l), (0x12, 0x12, 0x12));
    run(&mut c, 3);
    assert_eq!(c.mem.read(0xC000), 0x12);
    assert_eq!(c.regs.e, 0x12);
}

#[test]
fn ld_hl_inc_dec_addressing() {
    let mut c = cpu_with(&[0x21, 0x00, 0xC0, 0x3E, 0x07, 0x22, 0x22, 0x32, 0x2A]);
    // LD HL,$C000; LD A,7; LD (HL+),A x2; LD (HL-),A; LD A,(HL+)
    run(&mut c, 5);
    assert_eq!((c.mem.read(0xC000), c.mem.read(0xC001)), (7, 7));
    assert_eq!(c.regs.hl(), 0xC001); // (HL-) at $C002 left HL = $C001
    assert_eq!(c.mem.read(0xC002), 7);
    c.step();
    assert_eq!(c.regs.hl(), 0xC002);
}

#[test]
fn daa_matches_bcd_reference_for_add_and_sub() {
    fn bcd(v: u8) -> u8 { (v / 10) << 4 | (v % 10) }
    for a in 0..100u8 {
        for b in 0..100u8 {
            // ADD then DAA
            let mut c = cpu_with(&[0x3E, bcd(a), 0xC6, bcd(b), 0x27]);
            run(&mut c, 3);
            let sum = a as u16 + b as u16;
            assert_eq!(c.regs.a, bcd((sum % 100) as u8), "{a}+{b}");
            assert_eq!(c.regs.flag_c(), sum >= 100, "{a}+{b} carry");
            assert_eq!(c.regs.flag_z(), sum % 100 == 0);
            // SUB then DAA
            let mut c = cpu_with(&[0x3E, bcd(a), 0xD6, bcd(b), 0x27]);
            run(&mut c, 3);
            let diff = (a as i16 - b as i16).rem_euclid(100) as u8;
            assert_eq!(c.regs.a, bcd(diff), "{a}-{b}");
            assert_eq!(c.regs.flag_c(), a < b, "{a}-{b} borrow");
        }
    }
}

#[test]
fn accumulator_rotates_clear_z() {
    // RLCA on $85 -> $0B C
    let mut c = cpu_with(&[0x3E, 0x85, 0x07]);
    run(&mut c, 2);
    assert_eq!((c.regs.a, c.regs.f), (0x0B, FLAG_C));
    // RRA with carry clear on $01 -> $00 and C set; Z must stay clear
    let mut c = cpu_with(&[0x3E, 0x01, 0xB7, 0x1F]); // OR A clears C
    run(&mut c, 3);
    assert_eq!((c.regs.a, c.regs.f), (0x00, FLAG_C));
    // RLA through carry
    let mut c = cpu_with(&[0x37, 0x3E, 0x80, 0x17]);
    run(&mut c, 3);
    assert_eq!((c.regs.a, c.regs.f), (0x01, FLAG_C));
}

#[test]
fn cpl_scf_ccf() {
    let mut c = cpu_with(&[0x3E, 0x55, 0x2F, 0x37, 0x3F]);
    run(&mut c, 2);
    assert_eq!((c.regs.a, c.regs.f & (FLAG_N | FLAG_H)), (0xAA, FLAG_N | FLAG_H));
    c.step();
    assert_eq!(c.regs.f & (FLAG_N | FLAG_H | FLAG_C), FLAG_C);
    c.step();
    assert_eq!(c.regs.f & (FLAG_N | FLAG_H | FLAG_C), 0);
}

// ---- CB prefix --------------------------------------------------------------

#[test]
fn cb_shifts_rotates_swap() {
    let cases: &[(u8, u8, bool, u8, bool)] = &[
        // (cb opcode for B, input, carry_in, result, carry_out)
        (0x00, 0x85, false, 0x0B, true),  // RLC
        (0x08, 0x01, false, 0x80, true),  // RRC
        (0x10, 0x80, true, 0x01, true),   // RL
        (0x18, 0x01, true, 0x80, true),   // RR
        (0x20, 0xC0, false, 0x80, true),  // SLA
        (0x28, 0x81, false, 0xC0, true),  // SRA keeps the sign bit
        (0x30, 0xF1, false, 0x1F, false), // SWAP clears carry
        (0x38, 0x81, false, 0x40, true),  // SRL
    ];
    for &(cb, input, cin, out, cout) in cases {
        let setup = if cin { 0x37 } else { 0xB7 }; // SCF / OR A
        let mut c = cpu_with(&[setup, 0x06, input, 0xCB, cb]);
        run(&mut c, 3);
        assert_eq!(c.regs.b, out, "CB {cb:02X}");
        assert_eq!(c.regs.flag_c(), cout, "CB {cb:02X} carry");
        assert_eq!(c.regs.flag_z(), out == 0);
        assert!(!c.regs.flag_n() && !c.regs.flag_h());
    }
}

#[test]
fn cb_bit_res_set() {
    let mut c = cpu_with(&[0x06, 0x80, 0xCB, 0x78, 0xCB, 0x40, 0xCB, 0x80, 0xCB, 0xC1]);
    // LD B,$80; BIT 7,B; BIT 0,B; RES 0,B; SET 0,C
    run(&mut c, 2);
    assert_eq!(c.regs.f & (FLAG_Z | FLAG_H), FLAG_H, "bit 7 is set");
    c.step();
    assert_eq!(c.regs.f & (FLAG_Z | FLAG_H), FLAG_Z | FLAG_H, "bit 0 is clear");
    c.step();
    c.step();
    assert_eq!(c.regs.c, 0x13 | 1);
    assert_eq!(c.regs.b, 0x80);
}

#[test]
fn cb_on_hl_indirect_and_cycles() {
    let mut c = cpu_with(&[0x21, 0x00, 0xC0, 0xCB, 0xC6, 0xCB, 0x46, 0xCB, 0x3E]);
    // SET 0,(HL); BIT 0,(HL); SRL (HL)
    c.step();
    assert_eq!(c.step(), 16);
    assert_eq!(c.mem.read(0xC000), 1);
    assert_eq!(c.step(), 12);
    assert_eq!(c.step(), 16);
    assert_eq!(c.mem.read(0xC000), 0);
}

// ---- 16-bit, stack, control flow --------------------------------------------

#[test]
fn add_hl_flags_and_z_untouched() {
    let mut c = cpu_with(&[0x21, 0xFF, 0x0F, 0x01, 0x01, 0x00, 0x09]); // ADD HL,BC
    run(&mut c, 3);
    assert_eq!(c.regs.hl(), 0x1000);
    assert_eq!(c.regs.f & (FLAG_N | FLAG_H | FLAG_C), FLAG_H);
    assert!(c.regs.flag_z(), "Z from the reset value is preserved");
}

#[test]
fn inc_dec_rp_wrap_without_flags() {
    let mut c = cpu_with(&[0x01, 0xFF, 0xFF, 0x03, 0x0B]);
    let f0 = c.regs.f;
    run(&mut c, 2);
    assert_eq!(c.regs.bc(), 0);
    c.step();
    assert_eq!(c.regs.bc(), 0xFFFF);
    assert_eq!(c.regs.f, f0);
}

#[test]
fn add_sp_and_ld_hl_sp_flags_use_low_byte() {
    // LD SP,$00FF; ADD SP,1
    let mut c = cpu_with(&[0x31, 0xFF, 0x00, 0xE8, 0x01]);
    run(&mut c, 2);
    assert_eq!(c.regs.sp, 0x0100);
    assert_eq!(c.regs.f, FLAG_H | FLAG_C);
    // LD SP,$0100; LD HL,SP-1
    let mut c = cpu_with(&[0x31, 0x00, 0x01, 0xF8, 0xFF]);
    run(&mut c, 2);
    assert_eq!(c.regs.hl(), 0x00FF);
    assert_eq!(c.regs.f, 0, "low byte 00 + FF sets neither H nor C");
}

#[test]
fn push_pop_af_masks_low_nibble() {
    let mut c = cpu_with(&[0x01, 0xFF, 0x12, 0xC5, 0xF1]); // LD BC,$12FF; PUSH BC; POP AF
    run(&mut c, 3);
    assert_eq!(c.regs.a, 0x12);
    assert_eq!(c.regs.f, 0xF0);
}

#[test]
fn call_ret_and_rst() {
    let mut c = cpu_with(&[0xCD, 0x10, 0x01, 0x76]); // CALL $0110; HALT
    c.load(0x0110, &[0x3C, 0xC9]); // INC A; RET
    run(&mut c, 3);
    assert_eq!(c.regs.pc, 0x0103);
    assert_eq!(c.regs.a, 0x02);
    assert_eq!(c.regs.sp, 0xFFFE);
    let mut c = cpu_with(&[0xEF]); // RST $28
    c.step();
    assert_eq!((c.regs.pc, c.mem.read16(c.regs.sp)), (0x0028, 0x0101));
}

#[test]
fn conditional_ops_cycles_taken_and_not() {
    // JR NZ (Z set at reset) not taken: 8; JR Z taken: 12
    let mut c = cpu_with(&[0x20, 0x05, 0x28, 0x05]);
    assert_eq!(c.step(), 8);
    assert_eq!(c.step(), 12);
    assert_eq!(c.regs.pc, 0x0109);
    let mut c = cpu_with(&[0xC0, 0xC8]); // RET NZ (not taken), RET Z (taken)
    assert_eq!(c.step(), 8);
    c.regs.sp = 0xC000;
    assert_eq!(c.step(), 20);
    let mut c = cpu_with(&[0xC2, 0, 2, 0xCA, 0, 2]); // JP NZ no, JP Z yes
    assert_eq!(c.step(), 12);
    assert_eq!(c.step(), 16);
    assert_eq!(c.regs.pc, 0x0200);
    let mut c = cpu_with(&[0xC4, 0, 2, 0xCC, 0, 2]); // CALL NZ no, CALL Z yes
    assert_eq!(c.step(), 12);
    assert_eq!(c.step(), 24);
}

#[test]
fn jr_backwards_and_jp_hl() {
    let mut c = cpu_with(&[0x18, 0xFE]); // JR -2: jumps to itself
    c.step();
    assert_eq!(c.regs.pc, 0x0100);
    let mut c = cpu_with(&[0x21, 0x34, 0x12, 0xE9]);
    run(&mut c, 2);
    assert_eq!(c.regs.pc, 0x1234);
}

#[test]
fn ldh_and_c_forms_use_ff_page() {
    let mut c = cpu_with(&[0x3E, 0x99, 0xE0, 0x80, 0xAF, 0xF0, 0x80, 0x0E, 0x81, 0xE2, 0xF2]);
    // LD A,$99; LDH ($80),A; XOR A; LDH A,($80); LD C,$81; LD ($FF00+C),A; LD A,($FF00+C)
    run(&mut c, 2);
    assert_eq!(c.mem.read(0xFF80), 0x99);
    run(&mut c, 2);
    assert_eq!(c.regs.a, 0x99);
    run(&mut c, 3);
    assert_eq!(c.mem.read(0xFF81), 0x99);
}

#[test]
fn ld_nn_sp_and_ld_nn_a() {
    let mut c = cpu_with(&[0x08, 0x00, 0xC0, 0x3E, 0x5A, 0xEA, 0x10, 0xC0, 0xFA, 0x10, 0xC0]);
    c.step();
    assert_eq!(c.mem.read16(0xC000), 0xFFFE);
    run(&mut c, 2);
    assert_eq!(c.mem.read(0xC010), 0x5A);
    c.regs.a = 0;
    c.step();
    assert_eq!(c.regs.a, 0x5A);
}

#[test]
fn rom_region_is_read_only_to_programs() {
    let mut c = cpu_with(&[0x3E, 0x77, 0xEA, 0x00, 0x20]); // LD ($2000),A
    run(&mut c, 2);
    assert_eq!(c.mem.read(0x2000), 0);
    c.mem.poke(0x2000, 9);
    assert_eq!(c.mem.read(0x2000), 9);
}

#[test]
fn stop_is_two_bytes_and_halts() {
    let mut c = cpu_with(&[0x10, 0x00, 0x3C]);
    c.step();
    assert!(c.halted);
    assert_eq!(c.regs.pc, 0x0102);
}

#[test]
fn ei_then_di_never_enables() {
    let mut c = cpu_with(&[0xFB, 0xF3, 0x00]);
    run(&mut c, 3);
    assert!(!c.ime);
}

#[test]
fn wrapping_sp_and_pc_do_not_panic() {
    let mut c = cpu_with(&[0xC5]);
    c.regs.sp = 0x0001;
    c.step();
    assert_eq!(c.regs.sp, 0xFFFF);
    let mut c = Cpu::new();
    c.regs.pc = 0xFFFF;
    c.step(); // fetches $00 NOP at $FFFF and wraps
    assert_eq!(c.regs.pc, 0x0000);
}

// ---- disassembler -----------------------------------------------------------

#[test]
fn disasm_length_matches_executor_for_every_opcode() {
    for op in 0..=255u16 {
        let mut c = Cpu::new();
        c.regs.pc = 0xC000;
        c.mem.poke(0xC000, op as u8);
        for (i, v) in [0x11u8, 0x22].iter().enumerate() {
            c.mem.poke(0xC001 + i as u16, *v);
        }
        let (_, len) = disassemble(&c.mem, 0xC000);
        c.step();
        if c.fault.is_some() {
            assert_eq!(len, 1, "illegal {op:02X} disassembles as one byte");
        } else {
            assert_eq!(c.last_len, len, "opcode {op:02X}");
        }
    }
}

#[test]
fn disasm_length_matches_for_all_cb_opcodes() {
    for cb in 0..=255u16 {
        let mut c = Cpu::new();
        c.regs.pc = 0xC000;
        c.mem.poke(0xC000, 0xCB);
        c.mem.poke(0xC001, cb as u8);
        let (text, len) = disassemble(&c.mem, 0xC000);
        c.step();
        assert_eq!((c.last_len, len), (2, 2), "{text}");
    }
}

#[test]
fn disasm_text_samples() {
    let mut c = Cpu::new();
    let cases: &[(&[u8], &str)] = &[
        (&[0x00], "NOP"),
        (&[0x3C], "INC A"),
        (&[0x41], "LD B,C"),
        (&[0x71], "LD (HL),C"),
        (&[0x76], "HALT"),
        (&[0x21, 0x00, 0xC0], "LD HL,$C000"),
        (&[0x20, 0xFB], "JR NZ,$0000"),
        (&[0xC3, 0x50, 0x01], "JP $0150"),
        (&[0xCB, 0x7C], "BIT 7,H"),
        (&[0xE0, 0x80], "LDH ($80),A"),
        (&[0xF8, 0xFF], "LD HL,SP-1"),
        (&[0xFE, 0x0B], "CP $0B"),
        (&[0xFF], "RST $38"),
        (&[0xD3], "DB $D3"),
    ];
    for (bytes, want) in cases {
        c.load(0x0100, bytes);
        c.mem.poke(0x0100 + bytes.len() as u16, 0);
        let (got, len) = disassemble(&c.mem, 0x0100);
        // JR target is absolute: 0x0102 - 5 = 0x00FD for the sample above.
        if *want == "JR NZ,$0000" {
            assert_eq!(got, "JR NZ,$00FD");
        } else {
            assert_eq!(&got, want);
        }
        assert_eq!(len as usize, bytes.len());
    }
}

#[test]
fn disassembling_at_ffff_does_not_panic() {
    let c = Cpu::new();
    let (_, len) = disassemble(&c.mem, 0xFFFF);
    assert!(len >= 1);
}

#[test]
fn demos_disassemble_with_no_unknown_bytes() {
    for d in DEMOS {
        let mut c = cpu_with(d.code);
        let mut addr = LOAD_ADDR;
        let end = LOAD_ADDR + d.code.len() as u16;
        while addr < end {
            let (text, len) = disassemble(&c.mem, addr);
            assert!(!text.starts_with("DB"), "{}: {text} at {addr:04X}", d.name);
            addr += len as u16;
        }
        assert_eq!(addr, end, "{} ends on an instruction boundary", d.name);
        c.step();
    }
}
