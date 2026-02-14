# Game Boy Emulator

A Sharp LR35902 CPU emulator and interactive debugger for the Precursor handheld.

## Features

- **CPU Core**: Emulates ~60 opcodes of the Game Boy's Sharp LR35902 processor
- **Registers**: Full A/F/B/C/D/E/H/L/SP/PC register set with flag display (Z/N/H/C)
- **Memory**: 64KB address space with hex memory viewer (8 bytes per row)
- **Disassembler**: Real-time disassembly view with PC highlight
- **Demo Programs**: 4 built-in programs — Counter, Add Two Numbers, Fibonacci, Memory Fill
- **Step Debugger**: Single-step execution or run-until-halt (max 1000 steps)
- **Memory Viewer**: Navigate memory in 16-byte or 256-byte increments

## Screens

1. **Demo Select** — Browse and load built-in demo programs
2. **Running** — CPU state display with registers, flags, and disassembly
3. **Memory** — Hex dump of memory contents with address navigation

## Controls

### Demo Select
- **Up/Down**: Browse demos
- **Enter**: Load selected demo
- **Menu**: Quit

### Running
- **Enter**: Single step
- **Space**: Run until halt
- **R**: Reset current demo
- **M**: Memory viewer
- **Menu**: Back to demo select

### Memory Viewer
- **Up/Down**: Scroll +/- 16 bytes
- **Left/Right**: Scroll +/- 256 bytes
- **Menu**: Back to running

## Architecture

```
src/
├── main.rs    — Xous event loop and GAM registration
├── cpu.rs     — LR35902 CPU core, memory, disassembler, demo programs
├── app.rs     — State machine (DemoSelect → Running ↔ Memory)
└── ui.rs      — Rendering: register display, disassembly, hex dump
```

## Implemented Opcodes

| Category | Instructions |
|----------|-------------|
| Load | LD r,n · LD r16,nn · LD (HL),r · LD r,(HL) · LD (nn),A |
| Arithmetic | ADD · SUB · AND · OR · XOR · CP · INC · DEC |
| 16-bit | INC r16 · DEC r16 · ADD HL,r16 |
| Control | JP · JR · CALL · RET · JP cc · JR cc · CALL cc · RET cc |
| Stack | PUSH · POP |
| Misc | NOP · HALT · CCF · SCF · CPL · DAA |

## Development

Built for the [Precursor](https://github.com/nicholasgasior/xous-dev-toolkit) hardware platform running Xous OS.

## Author

Tyler Colby

## License

Apache License 2.0
