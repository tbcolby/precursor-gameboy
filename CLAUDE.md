# Build Notes — Game Boy Emulator

## What This Is
Sharp LR35902 CPU emulator for Precursor. Implements ~60 opcodes with register/flag display, disassembler, 4 demo programs, and hex memory viewer.

## Architecture Decisions
- **No ROM loading**: Precursor has no filesystem access for GB ROMs, so built-in demo programs instead
- **64KB flat memory**: Single Vec<u8> rather than memory-mapped regions (no GPU/APU needed for CPU demos)
- **Disassembler**: Real-time disassembly from PC forward, not pre-computed
- **Step debugger pattern**: Single-step by default, space for run-until-halt with 1000-step safety limit
- **Flag computation**: Inline in each arithmetic opcode for accuracy (half-carry especially)

## Key Implementation Details
- Registers struct with bc()/de()/hl() pair accessors and set_bc()/set_de()/set_hl() mutators
- Flags stored in F register bits: Z=7, N=6, H=5, C=4 (Game Boy standard)
- Stack grows downward from 0xFFFE
- Demo programs loaded at 0x0100 (standard Game Boy entry point)
- Memory viewer defaults to 0xC000 (WRAM start)

## GAM API Pattern
Uses the standard simplified GAM API: gam.draw_rectangle(), gam.draw_line(), gam.post_textview(), gam.clear(), gam.redraw(). No Canvas/DrawStyle/PixelColor.

## Testing
Manual testing via demo programs — each exercises different opcode categories.
