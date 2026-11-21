//! Hand-assembled demo programs, loaded at $0100. Every one is run to HALT in
//! the tests and its result checked.

pub struct DemoProgram {
    pub name: &'static str,
    pub description: &'static str,
    pub code: &'static [u8],
}

/// Where demos are loaded (the cartridge entry point).
pub const LOAD_ADDR: u16 = 0x0100;

pub static DEMOS: &[DemoProgram] = &[
    DemoProgram {
        name: "Counter",
        description: "Count A from 0 to 11",
        code: &[
            0xAF, //       XOR A
            0x3C, // loop: INC A
            0xFE, 0x0B, // CP 11
            0x20, 0xFB, // JR NZ,loop
            0x76, //       HALT
        ],
    },
    DemoProgram {
        name: "Add Two Numbers",
        description: "25 + 37 = 62 ($3E) in A",
        code: &[
            0x3E, 0x19, // LD A,25
            0x06, 0x25, // LD B,37
            0x80, //       ADD A,B
            0x76, //       HALT
        ],
    },
    DemoProgram {
        name: "Fibonacci",
        description: "First 8 Fibonacci numbers at $C000",
        code: &[
            0x21, 0x00, 0xC0, //       LD HL,$C000
            0x06, 0x00, //             LD B,0   (previous)
            0x0E, 0x01, //             LD C,1   (current)
            0x16, 0x08, //             LD D,8   (count)
            0x71, //             loop: LD (HL),C
            0x23, //                   INC HL
            0x78, //                   LD A,B
            0x81, //                   ADD A,C
            0x41, //                   LD B,C
            0x4F, //                   LD C,A
            0x15, //                   DEC D
            0x20, 0xF7, //             JR NZ,loop
            0x76, //                   HALT
        ],
    },
    DemoProgram {
        name: "Memory Fill",
        description: "Fill $C000-$C0FF with $42",
        code: &[
            0x21, 0x00, 0xC0, //       LD HL,$C000
            0x3E, 0x42, //             LD A,$42
            0x77, //             loop: LD (HL),A
            0x2C, //                   INC L
            0x20, 0xFC, //             JR NZ,loop
            0x76, //                   HALT
        ],
    },
    DemoProgram {
        name: "BCD Add",
        description: "19 + 28 in BCD with DAA = $47",
        code: &[
            0x3E, 0x19, // LD A,$19
            0xC6, 0x28, // ADD A,$28
            0x27, //       DAA
            0x76, //       HALT
        ],
    },
];
