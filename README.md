# Precursor Game Boy CPU

A step debugger for the Game Boy's Sharp LR35902 CPU on [Precursor](https://precursor.dev). Load a built-in demo program, single-step it or run it, and watch registers, flags and the disassembly. It is a CPU and 64 KB of memory, **not a Game Boy emulator**: there is no cartridge loading, no screen output, no timer, no joypad and no sound.

## Features

- Complete LR35902 instruction set: all defined base opcodes and all 256 CB-prefixed ones. Flags (Z, N, H, C), DAA, ADC/SBC, 16-bit arithmetic, `ADD SP,e`, `LD HL,SP+e`, stack, CALL/RET/RST, conditional timing.
- Interrupts: IME with EI's one-instruction delay, IF/IE at `$FF0F`/`$FFFF`, dispatch to the five vectors, RETI.
- HALT: stops until an interrupt is pending (even with IME off, as on hardware). With nothing that can raise an interrupt the demo just halts.
- The 11 undefined opcodes stop the CPU and the header shows `FAULT: illegal opcode $xx at $pppp` (no silent NOPs).
- Disassembler built on the same decode as the executor; a test checks its instruction length against what the CPU consumes for every opcode.
- Five demos, each tested to halt with the right result: Counter, Add Two Numbers, Fibonacci (8 numbers at `$C000`), Memory Fill, BCD Add (DAA).
- Memory viewer with 8 bytes per row.

## Keys

| Screen | Keys |
|---|---|
| Demo list | Up/Down select, Enter load, Backspace or F4 leave the app |
| Debugger | Enter step, Space run (up to 10,000 instructions per press), r reset, m memory, Backspace or F4 back |
| Memory | Up/Down +-8 bytes, Left/Right +-256, p jump to PC, s jump to SP, c jump to `$C000`, Backspace or F4 back |

The menu key (centre of the D-pad) is consumed by Xous and never reaches the app.

## Limitations

- CPU only. Memory is flat 64 KB; writes below `$8000` are ignored by programs (ROM-like) but demos are loaded there by the debugger. No MBC, VRAM/OAM behaviour, echo RAM, DMA, timers, serial, or I/O registers other than IF/IE.
- No ROM loading. Programs are the built-in demos.
- Cycle counts follow the standard instruction timings; memory-access sub-cycle timing is not modelled.
- Not modelled: the HALT bug, STOP's clock/speed-switch behaviour (STOP just halts).
- Not run against Blargg's `cpu_instrs` (no ROM harness here); correctness is covered by the unit tests in `core/tests/cpu.rs` (flags, DAA against a BCD reference over all inputs, CB ops, stack, interrupts, timing).
- Screen layout hasn't been checked on a device yet. Renode capture not yet run: `capture.py` is written but not executed; verification so far is the hosted build, clippy, lint and host tests.

## Building

Builds inside a [xous-core](https://github.com/betrusted-io/xous-core) tree. With [xous-dev-toolkit](https://github.com/tbcolby/xous-dev-toolkit):

```bash
bin/integrate --xous-root ~/xous-core --grade ~/Dev/precursor/precursor-gameboy
```

Host tests alone: `cd core && cargo test`.

## Layout

```
core/src/cpu.rs       registers, memory, interrupts, full decode and execute
core/src/disasm.rs    disassembler
core/src/demos.rs     demo programs
core/src/session.rs   step / run / reset / listing for the shell
core/tests/cpu.rs     instruction, flag, interrupt and demo tests
src/main.rs, app.rs   GAM registration, key mapping, drawing
capture.py            Renode capture sequence (not yet run)
```

## Author

Tyler Colby, [Colby's Data Movers, LLC](https://colbysdatamovers.com). Licensed under the Apache License, Version 2.0 (see [LICENSE](LICENSE)).
