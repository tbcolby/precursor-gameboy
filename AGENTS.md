# Agent Evolution Report — Game Boy Emulator

## App #31 of 32 | Wave 7 (Apps 29–32)

### Build Agent
- **Model**: Claude Opus 4.6 (main context)
- **Approach**: Direct build from main context (not delegated to subagent)
- **Reason**: Previous subagent attempts failed due to permission restrictions on Bash/Write tools

### Design Decisions
1. **CPU architecture**: Chose flat opcode table over decoded instruction structs — simpler for ~60 opcodes
2. **Memory model**: Single 64KB Vec vs memory-mapped regions — sufficient for CPU-only demos
3. **Demo programs**: Hand-assembled byte arrays rather than an assembler — keeps code self-contained
4. **Disassembler**: Forward-scan from PC rather than full binary analysis — matches step-debugger use case

### What Worked
- LR35902 opcode encoding is regular enough for a match-based decoder
- Disassembly view with PC highlight gives immediate feedback
- Memory viewer with hex dump is useful for verifying demo program behavior
- 4 demos (Counter, Add Two Numbers, Fibonacci, Memory Fill) exercise all major opcode categories

### What Could Be Better
- CB-prefix opcodes (bit ops, rotates) not implemented
- No interrupts or timer registers
- No GPU/PPU — can't render Game Boy graphics
- No sound APU
- Could add breakpoint support

### Lines of Code
- cpu.rs: ~350 LOC (CPU core + disassembler + demos)
- app.rs: ~130 LOC (state machine)
- ui.rs: ~160 LOC (rendering)
- main.rs: ~43 LOC (event loop)
- **Total: ~683 LOC**

### Agent Pattern Notes
- Direct build continues to be the only reliable approach (subagents lack tool permissions)
- CPU emulator is the most technically complex app in the ecosystem
- Reused proven GAM API pattern from all previous apps
