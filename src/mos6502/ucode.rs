#![allow(dead_code)]

/* Operations I haven't added support for, but need to

PC ; R ; fetch low address byte, increment PC
PC ; R ; copy low address byte to PCL, fetch high address byte to PCH
PC ; R ; fetch high byte of address, add index register X to low address byte, increment PC
PC ; R ; Fetch opcode of next instruction, If branch is taken, add operand to PCL. Otherwise increment PC.
PC* ; R ; Fetch opcode of next instruction. Fix PCH. If it did not change, increment PC.
PC ; R ; Fetch opcode of next instruction, increment PC.
pointer ; R ; fetch low address to latch
pointer+1* ; R ; fetch PCH, copy latch to PCL
PC+1 ; R ; fetch address low
PC+2 ; R ; fetch address high
$D019 ; R ; read memory
$D019 ; W ; write the value back, rotate right
$D019 ; W ; write the new value back
PC+2 ; R ; fetch address high, add X to address low
$DC0D ; R ; read from address, fix high byte of address
$DD0D ; R ; read from right address
$DE0D ; W ; write to right address
*/

#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub enum uCode{
  Unused,

  FetchOpcode,         // PC ; R ; pc++, ucode_curr=decoded_fetch  ; fetch opcode, increment PC
  DummyReadNext_IncPC, // PC ; R ; pc++                            ; read next instruction byte (and throw it away), increment PC
  DummyReadNext,       // PC ; R ;                                 ; read next instruction byte (and throw it away)
  IncPC,               // PC ; R ; pc++                            ; increment PC
  FetchValue,          // PC ; R ; pc++, data=value                ; fetch value, increment PC
  FetchOperand,        // PC ; R ; pc++, data=operand              ; fetch operand, increment PC

  PushPCH,       // $0100,S ; W ; sp--         ; push PCH on stack, decrement S
  PushPCL,       // $0100,S ; W ; sp--         ; push PCL on stack, decrement S
  PushPS,        // $0100,S ; W ; sp--         ; push P on stack (with B flag set), decrement S
  PushReg(char), // $0100,S ; W ; sp--         ; push register on stack, decrement S
  IncSP,         // $0100,S ; R ; sp++         ; increment S
  PullReg(char), // $0100,S ; R ; a|ps=pop     ; pull register from stack
  PullPS,        // $0100,S ; R ; ps=pop,  s++ ; pull P from stack, increment S
  PullPCL,       // $0100,S ; R ; pcl=pop, s++ ; pull PCL from stack, increment S
  PullPCH,       // $0100,S ; R ; pch=pop      ; pull PCH from stack
  StackInternal, // $0100,S ; R ; s--          ; internal operation (predecrement S?)
  CopyPCL_FetchAddrHighPCH, // PC ; R ; pcl=addr_low, addr_high=fetch ; copy low address byte to PCL, fetch high address byte to PCH

  FetchPCL, // $FFFE ; R ; pc_low=fetch_FFFE ; fetch PCL
  FetchPCH, // $FFFF ; R ; pc_high=fetch_FFFF ; fetch PCH

  FetchAddrLow,               // PC        ; R ; addr_low=fetch                 ; fetch low byte of address, increment PC
  FetchAddrHigh,              // PC        ; R ; addr_high=fetch                ; fetch high byte of address, increment PC
  FetchAddrHigh_AddReg(char), // PC        ; R ; addr_high=fetch, addr_low+=x|y ; fetch high byte of address, add index register to low address byte, increment PC
  FetchAddr,                  // PC        ; R ; addr=fetch                     ; fetch address, increment PC
  
  FetchPtrAddrLow,            // PC        ; R ; ptr_low=fetch                        ; fetch pointer address low, increment PC
  FetchPtrAddrHigh,           // PC        ; R ; ptr_high=fetch                       ; fetch pointer address high, increment PC
  FetchPtrAddr,               // PC        ; R ; ptr=fetch                            ; fetch pointer address, increment PC
  FetchEffAddrLow,            // pointer+X ; R ; addr_low=read_ptr_low                ; fetch effective address low 
  FetchEffAddrHigh,           // pointer+X ; R ; addr_high=read_ptr_high              ; fetch effective address high
  FetchEffAddrHigh_AddY,      // pointer+1 ; R ; addr_high=read_ptr_high, addr_low+=y ; fetch effective address high, add Y to low byte of effective address
  ReadValue_PtrAddX,          // pointer   ; R ; value=read, ptr_low+=x               ; read from the address, add X to it
  
  ReadValue,              // address ; R ; data=read                               ; read from effective address
  ReadValue_AddReg(char), // address ; R ; data=read+x|y                           ; read from address, add index register to it
  ReadValue_Fix,          // address ; R ; data=read,    addr_high=fixed_addr_high ; read from effective address, fix the high byte of effective address

  WriteValue,     // address ; W ; mem[addr]=value ; write to effective address
  WriteReg(char), // address ; W ; mem[addr]=a|x|y ; write register to effective address

  // Offical opcodes
  // PHA, PLA, PHP, and PLP are not actually used. PushReg and PullReg above are used instead
  LDA, STA, LDX, STX, LDY, STY,
  TAX, TXA, TAY, TYA,
  ADC, SBC, INC, DEC, INX, DEX, INY, DEY,
  ASL, LSR, ROL, ROR,
  AND, ORA, EOR, BIT,
  CMP, CPX, CPY,
  BCC, BCS, BEQ, BNE, BPL, BMI, BVC, BVS,
  JMP, JSR, RTS, BRK, RTI,
  PHA, PLA, PHP, PLP, TXS, TSX,
  CLC, SEC, CLI, SEI, CLD, SED, CLV,
  NOP,

  // Unnoffical opcodes
  UN_NOP,
  UN_STP,
  UN_SLO,
  UN_RLA,
  UN_SRE,
  UN_RRA,
  UN_SAX,
  UN_LAX,
  UN_DCP,
  UN_ISC,
  UN_ANC,
  UN_ALR,
  UN_ARR,
  UN_XAA,
  UN_AXS,
  UN_SBC,
  UN_AHX
}

macro_rules! uCodeBRK {
  () => { [uCode::DummyReadNext_IncPC, uCode::PushPCH, uCode::PushPCL, uCode::PushPS, uCode::FetchPCL, uCode::FetchPCH, uCode::BRK, uCode::FetchOpcode, uCode::FetchOpcode] }
}

macro_rules! uCodeJSR {
  () => { [uCode::FetchAddrLow, uCode::StackInternal, uCode::PushPCH, uCode::PushPCL, uCode::CopyPCL_FetchAddrHighPCH, uCode::JSR, uCode::FetchOpcode, uCode::Unused, uCode::Unused] }
}

macro_rules! uCodeRTI {
  () => { [uCode::DummyReadNext, uCode::IncSP, uCode::PullPS, uCode::PullPCL, uCode::PullPCH, uCode::RTI, uCode::FetchOpcode, uCode::Unused, uCode::Unused] }
}

macro_rules! uCodeRTS {
  () => { [uCode::DummyReadNext, uCode::IncSP, uCode::PullPCL, uCode::PullPCH, uCode::IncPC, uCode::RTS, uCode::FetchOpcode, uCode::Unused, uCode::Unused] }
}

macro_rules! uCodePLA_PLP {
  (uCode::PLP) => { [uCode::DummyReadNext, uCode::IncSP, uCode::PullReg('p'), uCode::FetchOpcode, uCode::CopyPCL_FetchAddrHighPCH, uCode::JSR, uCode::FetchOpcode, uCode::Unused, uCode::Unused] };
  (uCode::PLA) => { [uCode::DummyReadNext, uCode::IncSP, uCode::PullReg('a'), uCode::FetchOpcode, uCode::CopyPCL_FetchAddrHighPCH, uCode::JSR, uCode::FetchOpcode, uCode::Unused, uCode::Unused] };
}

macro_rules! uCodePHA_PHP {
  (uCode::PHP) => { [uCode::DummyReadNext, uCode::PushReg('p'), uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused] };
  (uCode::PHA) => { [uCode::DummyReadNext, uCode::PushReg('a'), uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused] };
}

macro_rules! uCodeAc_Ap {
  ($op:expr) => { [uCode::DummyReadNext, $op, uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused] };
}

macro_rules! uCodeI {
  ($op:expr) => { [uCode::FetchValue, $op, uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused] };
}

macro_rules! uCodeAb {
  (w, $op:expr)   => { [uCode::FetchAddrLow, uCode::FetchAddrHigh, $op,              uCode::WriteValue, uCode::FetchOpcode, uCode::Unused,     uCode::Unused,      uCode::Unused, uCode::Unused] };
  (r, $op:expr)   => { [uCode::FetchAddrLow, uCode::FetchAddrHigh, uCode::ReadValue, $op,               uCode::FetchOpcode, uCode::Unused,     uCode::Unused,      uCode::Unused, uCode::Unused] };
  (rmw, $op:expr) => { [uCode::FetchAddrLow, uCode::FetchAddrHigh, uCode::ReadValue, uCode::WriteValue, $op,                uCode::WriteValue, uCode::FetchOpcode, uCode::Unused, uCode::Unused] };
}

macro_rules! uCodeZP {
  (w, $op:expr)   => { [uCode::FetchAddr, $op,              uCode::WriteValue, uCode::FetchOpcode, uCode::Unused,     uCode::Unused,      uCode::Unused, uCode::Unused, uCode::Unused] };
  (r, $op:expr)   => { [uCode::FetchAddr, uCode::ReadValue, $op,               uCode::FetchOpcode, uCode::Unused,     uCode::Unused,      uCode::Unused, uCode::Unused, uCode::Unused] };
  (rmw, $op:expr) => { [uCode::FetchAddr, uCode::ReadValue, uCode::WriteValue, $op,                uCode::WriteValue, uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused] };
}

macro_rules! uCodeZPI {
  (w, $reg:expr, $op:expr) => { [uCode::FetchAddr, uCode::ReadValue_AddReg($reg), $op,              uCode::WriteValue, uCode::FetchOpcode, uCode::Unused,     uCode::Unused,      uCode::Unused, uCode::Unused] };
  (r, $reg:expr, $op:expr) => { [uCode::FetchAddr, uCode::ReadValue_AddReg($reg), uCode::ReadValue, $op,               uCode::FetchOpcode, uCode::Unused,     uCode::Unused,      uCode::Unused, uCode::Unused] };
  (rmw, $op:expr)          => { [uCode::FetchAddr, uCode::ReadValue_AddReg('x'), uCode::ReadValue, uCode::WriteValue,  $op,                uCode::WriteValue, uCode::FetchOpcode, uCode::Unused, uCode::Unused] };
}

macro_rules! uCodeAbI {
  (w, $reg:expr, $op:expr) => { [uCode::FetchAddrLow, uCode::FetchAddrHigh_AddReg($reg), uCode::ReadValue_Fix, $op,              uCode::WriteValue, uCode::FetchOpcode, uCode::Unused,     uCode::Unused,      uCode::Unused] };
  (r, $reg:expr, $op:expr) => { [uCode::FetchAddrLow, uCode::FetchAddrHigh_AddReg($reg), uCode::ReadValue_Fix, uCode::ReadValue, $op,               uCode::FetchOpcode, uCode::Unused,     uCode::Unused,      uCode::Unused] };
  (rmw, $op:expr)          => { [uCode::FetchAddrLow, uCode::FetchAddrHigh_AddReg('x'),  uCode::ReadValue_Fix, uCode::ReadValue, uCode::WriteValue, $op,                uCode::WriteValue, uCode::FetchOpcode, uCode::Unused] };
}

macro_rules! uCodeIIn {
  (w, $op:expr)   => { [uCode::FetchPtrAddr, uCode::ReadValue_PtrAddX, uCode::FetchEffAddrLow, uCode::FetchEffAddrHigh, $op,              uCode::WriteValue, uCode::FetchOpcode,     uCode::Unused,     uCode::Unused] };
  (r, $op:expr)   => { [uCode::FetchPtrAddr, uCode::ReadValue_PtrAddX, uCode::FetchEffAddrLow, uCode::FetchEffAddrHigh, uCode::ReadValue, $op,               uCode::FetchOpcode,     uCode::Unused,     uCode::Unused] };
  (rmw, $op:expr) => { [uCode::FetchPtrAddr, uCode::ReadValue_PtrAddX, uCode::FetchEffAddrLow, uCode::FetchEffAddrHigh, uCode::ReadValue, uCode::WriteValue, $op,                    uCode::WriteValue, uCode::FetchOpcode] };
}

macro_rules! uCodeInI {
  (w, $op:expr)   => { [uCode::FetchPtrAddr, uCode::FetchEffAddrLow, uCode::FetchEffAddrHigh_AddY, uCode::ReadValue_Fix, $op,               uCode::WriteValue, uCode::FetchOpcode, uCode::Unused,      uCode::Unused] };
  (r, $op:expr)   => { [uCode::FetchPtrAddr, uCode::FetchEffAddrLow, uCode::FetchEffAddrHigh_AddY, uCode::ReadValue_Fix, uCode::ReadValue,  $op,               uCode::FetchOpcode, uCode::Unused,      uCode::Unused] };
  (rmw, $op:expr) => { [uCode::FetchPtrAddr, uCode::FetchEffAddrLow, uCode::FetchEffAddrHigh_AddY, uCode::ReadValue,     uCode::WriteValue, $op,               uCode::WriteValue,  uCode::FetchOpcode, uCode::Unused] };
}

#[allow(non_camel_case_types)]
pub type uCodeTable = [[uCode; 9]; 256];

// Testing: Written at 5am, full of mistakes
//  no indent = confirmed valid
//  indent = still checking validity
pub const DEFAULT_UCODE: uCodeTable = [
/* $00 */ uCodeBRK!(),
/* $01 */ uCodeIIn!(r, uCode::ORA),
    /* 02 */ [uCode::UN_STP; 9],
    /* 03 */ uCodeIIn!(rmw, uCode::UN_SLO),
    /* 04 */ uCodeZP!(r, uCode::UN_NOP),
/* $05 */ uCodeZP!(r, uCode::ORA),
/* $06 */ uCodeZP!(rmw, uCode::ASL),
    /* 07 */ uCodeZP!(rmw, uCode::UN_SLO),
/* $08 */ uCodeAc_Ap!(uCode::PHP),
/* $09 */ uCodeI!(uCode::ORA),
/* $0a */ uCodeAc_Ap!(uCode::ASL),
    /* 0B */ uCodeI!(uCode::UN_ANC),
    /* 0C */ uCodeAb!(r, uCode::UN_NOP),
/* $0d */ uCodeAb!(r, uCode::ORA),
/* $0E */ uCodeAb!(rmw, uCode::ASL),
    /* 0F */ uCodeAb!(rmw, uCode::UN_SLO),

    /* 10 */ [uCode::FetchOperand, uCode::BPL, uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused],
/* $11 */ uCodeInI!(r, uCode::ORA),
    /* 12 */ [uCode::UN_STP; 9],
    /* 13 */ uCodeInI!(rmw, uCode::UN_SLO),
    /* 14 */ uCodeZPI!(r, 'x', uCode::UN_NOP),
/* $15 */ uCodeZPI!(r, 'x', uCode::ORA),
/* $16 */ uCodeZPI!(rmw, uCode::ASL),
    /* 17 */ uCodeZPI!(rmw, uCode::UN_SLO),
/* $18 */ uCodeAc_Ap!(uCode::CLC),
/* $19 */ uCodeAbI!(r, 'y', uCode::ORA),
    /* 1A */ [uCode::DummyReadNext, uCode::UN_NOP, uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused],
    /* 1B */ uCodeAbI!(rmw, uCode::UN_SLO),
    /* 1C */ uCodeAbI!(r, 'x', uCode::UN_NOP),
/* $1d */ uCodeAbI!(r, 'x', uCode::ORA),
/* $1E */ uCodeAbI!(rmw, uCode::ASL),
    /* 1F */ uCodeAbI!(rmw, uCode::UN_SLO),

/* $20 */ uCodeJSR!(),
/* $21 */ uCodeIIn!(r, uCode::AND),
    /* 22 */ [uCode::UN_STP; 9],
    /* 23 */ uCodeIIn!(rmw, uCode::UN_RLA),
/* $24 */ uCodeZP!(r, uCode::BIT),
/* $25 */ uCodeZP!(r, uCode::AND),
/* $26 */ uCodeZP!(rmw, uCode::ROL),
    /* 27 */ uCodeZP!(rmw, uCode::UN_RLA),
/* $28 */ uCodePLA_PLP!(uCode::PLP),
/* $29 */ uCodeI!(uCode::AND),
/* $2a */ uCodeAc_Ap!(uCode::ROL),
    /* 2B */ uCodeI!(uCode::UN_ANC),
/* $2c */ uCodeAb!(r, uCode::BIT),
/* $2D */ uCodeAb!(r, uCode::AND),
/* $2e */ uCodeAb!(rmw, uCode::ROL),
    /* 2F */ uCodeAb!(rmw, uCode::UN_RLA),

    /* 30 */ [uCode::FetchOperand, uCode::BMI, uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused],
/* $31 */ uCodeInI!(r, uCode::AND),
    /* 32 */ [uCode::UN_STP; 9],
    /* 33 */ uCodeInI!(rmw, uCode::UN_RLA),
    /* 34 */ uCodeZPI!(r, 'x', uCode::UN_NOP),
/* $35 */ uCodeZPI!(r, 'x', uCode::AND),
/* $36 */ uCodeZPI!(rmw, uCode::ROL),
    /* 37 */ uCodeZPI!(rmw, uCode::UN_RLA),
/* $38 */ uCodeAc_Ap!(uCode::SEC),
/* $39 */ uCodeAbI!(r, 'y', uCode::AND),
    /* 3A */ [uCode::DummyReadNext, uCode::UN_NOP, uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused],
    /* 3B */ uCodeAbI!(rmw, uCode::UN_RLA),
    /* 3C */ uCodeAbI!(r, 'x', uCode::UN_NOP),
/* $3d */ uCodeAbI!(r, 'x', uCode::AND),
/* $3e */ uCodeAbI!(rmw, uCode::ROL),
    /* 3F */ uCodeAbI!(rmw, uCode::UN_RLA),

/* $40 */ uCodeRTI!(),
/* $41 */ uCodeIIn!(r, uCode::EOR),
    /* 42 */ [uCode::UN_STP; 9],
    /* 43 */ uCodeIIn!(rmw, uCode::UN_SRE),
    /* 44 */ uCodeZP!(r, uCode::UN_NOP),
/* $45 */ uCodeZP!(r, uCode::EOR),
/* $46 */ uCodeZP!(rmw, uCode::LSR),
    /* 47 */ uCodeZP!(rmw, uCode::UN_SRE),
/* $48 */ uCodePHA_PHP!(uCode::PHA),
/* $49 */ uCodeI!(uCode::EOR),
/* $4a */ uCodeAc_Ap!(uCode::LSR),
    /* 4B */ uCodeI!(uCode::UN_ALR),
    /* 4C */ [uCode::FetchAddrLow, uCode::FetchAddrHigh, uCode::JMP, uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused],
/* $4d */ uCodeAb!(r, uCode::EOR),
/* $4e */ uCodeAb!(rmw, uCode::LSR),
    /* 4F */ uCodeAb!(rmw, uCode::UN_SRE),

    /* 50 */ [uCode::FetchOperand, uCode::BVC, uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused],
/* $51 */ uCodeInI!(r, uCode::EOR),
    /* 52 */ [uCode::UN_STP; 9],
    /* 53 */ uCodeInI!(rmw, uCode::UN_SRE),
    /* 54 */ uCodeZPI!(r, 'x', uCode::UN_NOP),
/* $55 */ uCodeZPI!(r, 'x', uCode::EOR),
/* $56 */ uCodeZPI!(rmw, uCode::LSR),
    /* 57 */ uCodeZPI!(rmw, uCode::UN_SRE),
/* $58 */ uCodeAc_Ap!(uCode::CLI),
/* $59 */ uCodeAbI!(r, 'y', uCode::EOR),
    /* 5A */ [uCode::DummyReadNext, uCode::UN_NOP, uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused],
    /* 5B */ uCodeAbI!(rmw, uCode::UN_SRE),
    /* 5C */ uCodeAbI!(r, 'x', uCode::UN_NOP),
/* $5d */ uCodeAbI!(r, 'x', uCode::EOR),
/* $5e */ uCodeAbI!(rmw, uCode::LSR),
    /* 5F */ uCodeAbI!(rmw, uCode::UN_SRE),

/* $60 */ uCodeRTS!(),
/* $61 */ uCodeIIn!(r, uCode::ADC),
    /* 62 */ [uCode::UN_STP; 9],
    /* 63 */ uCodeIIn!(rmw, uCode::UN_RRA),
    /* 64 */ uCodeZP!(r, uCode::UN_NOP),
/* $65 */ uCodeZP!(r, uCode::ADC),
/* $66 */ uCodeZP!(rmw, uCode::ROR),
    /* 67 */ uCodeZP!(rmw, uCode::UN_RRA),
/* $68 */ uCodePLA_PLP!(uCode::PLA),
/* $69 */ uCodeI!(uCode::ADC),
/* $6A */ uCodeAc_Ap!(uCode::ROR),
    /* 6B */ uCodeI!(uCode::UN_ARR),
    /* 6C */ [uCode::FetchAddrLow, uCode::FetchAddrHigh, uCode::FetchEffAddrLow, uCode::FetchEffAddrHigh, uCode::JMP, uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused],
/* $6d */ uCodeAb!(r, uCode::ADC),
/* $6e */ uCodeAb!(rmw, uCode::ROR),
    /* 6F */ uCodeAb!(rmw, uCode::UN_RRA),

    /* 70 */ [uCode::FetchOperand, uCode::BVS, uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused],
/* $71 */ uCodeInI!(r, uCode::ADC),
    /* 72 */ [uCode::UN_STP; 9],
    /* 73 */ uCodeInI!(rmw, uCode::UN_RRA),
    /* 74 */ uCodeZPI!(r, 'x', uCode::UN_NOP),
/* $75 */ uCodeZPI!(r, 'x', uCode::ADC),
/* $76 */ uCodeZPI!(rmw, uCode::ROR),
    /* 77 */ uCodeZPI!(rmw, uCode::UN_RRA),
/* $78 */ uCodeAc_Ap!(uCode::SEI),
/* $79 */ uCodeAbI!(r, 'y', uCode::ADC),
    /* 7A */ [uCode::DummyReadNext, uCode::UN_NOP, uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused],
    /* 7B */ uCodeAbI!(rmw, uCode::UN_RRA),
    /* 7C */ uCodeAbI!(r, 'x', uCode::UN_NOP),
/* $7d */ uCodeAbI!(r, 'x', uCode::ADC),
/* 7e */ uCodeAbI!(rmw, uCode::ROR),
    /* 7F */ uCodeAbI!(rmw, uCode::UN_RRA),

    /* 80 */ uCodeI!(uCode::UN_NOP),
/* $81 */ uCodeIIn!(w, uCode::STA),
    /* 82 */ uCodeI!(uCode::UN_NOP),
    /* 83 */ uCodeIIn!(w, uCode::UN_SAX),
/* $84 */ uCodeZP!(w, uCode::STY),
/* $85 */ uCodeZP!(w, uCode::STA),
/* $86 */ uCodeZP!(w, uCode::STX),
    /* 87 */ uCodeZP!(w, uCode::UN_SAX),
/* $88 */ uCodeAc_Ap!(uCode::DEY),
    /* 89 */ uCodeI!(uCode::UN_NOP),
/* 8a */ uCodeAc_Ap!(uCode::TXA),
    /* 8B */ uCodeI!(uCode::UN_XAA),
/* $8C */ uCodeAb!(w, uCode::STY),
/* $8D */ uCodeAb!(w, uCode::STA),
/* $8E */ uCodeAb!(w, uCode::STX),
    /* 8F */ uCodeAb!(w, uCode::UN_SAX),

    /* 90 */ [uCode::FetchOperand, uCode::BCC, uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused],
/* $91 */ uCodeInI!(w, uCode::STA),
    /* 92 */ [uCode::UN_STP; 9],
    /* 93 */ uCodeInI!(w, uCode::UN_AHX),
/* $94 */ uCodeZPI!(w, 'x', uCode::STY),
/* $95 */ uCodeZPI!(w, 'x', uCode::STA),
/* $96 */ uCodeZPI!(w, 'y', uCode::STX),
    /* 97 */ uCodeZPI!(w, 'y', uCode::UN_SAX),
    /* 98 */ [uCode::DummyReadNext, uCode::TYA, uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused],
    /* 99 */ uCodeAbI!(w, 'y', uCode::STA),
    /* 9A */ [uCode::DummyReadNext, uCode::TXS, uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused],
    /* 9B */ uCodeAbI!(w, 'y', uCode::UN_AHX), // Actually TAS
    /* 9C */ uCodeAbI!(w, 'x', uCode::UN_NOP), // Actually SHY
    /* 9D */ uCodeAbI!(w, 'x', uCode::STA),
    /* 9E */ uCodeAbI!(w, 'y', uCode::UN_NOP), // Actually SHX
    /* 9F */ uCodeAbI!(w, 'y', uCode::UN_AHX),

    /* A0 */ uCodeI!(uCode::LDY),
    /* A1 */ uCodeIIn!(r, uCode::LDA),
    /* A2 */ uCodeI!(uCode::LDX),
    /* A3 */ uCodeIIn!(r, uCode::UN_LAX),
    /* A4 */ uCodeZP!(r, uCode::LDY),
    /* A5 */ uCodeZP!(r, uCode::LDA),
    /* A6 */ uCodeZP!(r, uCode::LDX),
    /* A7 */ uCodeZP!(r, uCode::UN_LAX),
    /* A8 */ [uCode::DummyReadNext, uCode::TAY, uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused],
    /* A9 */ uCodeI!(uCode::LDA),
    /* AA */ [uCode::DummyReadNext, uCode::TAX, uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused],
    /* AB */ uCodeI!(uCode::UN_LAX), // Actually ATX
    /* AC */ uCodeAb!(r, uCode::LDY),
    /* AD */ uCodeAb!(r, uCode::LDA),
    /* AE */ uCodeAb!(r, uCode::LDX),
    /* AF */ uCodeAb!(r, uCode::UN_LAX),

    /* B0 */ [uCode::FetchOperand, uCode::BCS, uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused],
    /* B1 */ uCodeInI!(r, uCode::LDA),
    /* B2 */ [uCode::UN_STP; 9],
    /* B3 */ uCodeInI!(r, uCode::UN_LAX),
    /* B4 */ uCodeZPI!(r, 'x', uCode::LDY),
    /* B5 */ uCodeZPI!(r, 'x', uCode::LDA),
    /* B6 */ uCodeZPI!(r, 'y', uCode::LDX),
    /* B7 */ uCodeZPI!(r, 'y', uCode::UN_LAX),
/* $b8 */ uCodeAc_Ap!(uCode::CLV),
    /* B9 */ uCodeAbI!(r, 'y', uCode::LDA),
    /* BA */ [uCode::DummyReadNext, uCode::TSX, uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused],
    /* BB */ uCodeAbI!(r, 'y', uCode::UN_LAX), // LAS
    /* BC */ uCodeAbI!(r, 'x', uCode::LDY),
    /* BD */ uCodeAbI!(r, 'x', uCode::LDA),
    /* BE */ uCodeAbI!(r, 'y', uCode::LDX),
    /* BF */ uCodeAbI!(r, 'y', uCode::UN_LAX),

    /* C0 */ uCodeI!(uCode::CPY),
    /* C1 */ uCodeIIn!(r, uCode::CMP),
    /* C2 */ uCodeI!(uCode::UN_NOP),
    /* C3 */ uCodeIIn!(rmw, uCode::UN_DCP),
    /* C4 */ uCodeZP!(r, uCode::CPY),
    /* C5 */ uCodeZP!(r, uCode::CMP),
    /* C6 */ uCodeZP!(rmw, uCode::DEC),
    /* C7 */ uCodeZP!(rmw, uCode::UN_DCP),
    /* C8 */ [uCode::DummyReadNext, uCode::INY, uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused],
    /* C9 */ uCodeI!(uCode::CMP),
    /* CA */ [uCode::DummyReadNext, uCode::DEX, uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused],
    /* CB */ uCodeI!(uCode::UN_AXS),
    /* CC */ uCodeAb!(r, uCode::CPY),
    /* CD */ uCodeAb!(r, uCode::CMP),
    /* CE */ uCodeAb!(rmw, uCode::DEC),
    /* CF */ uCodeAb!(rmw, uCode::UN_DCP),

    /* D0 */ [uCode::FetchOperand, uCode::BNE, uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused],
    /* D1 */ uCodeInI!(r, uCode::CMP),
    /* D2 */ [uCode::UN_STP; 9],
    /* D3 */ uCodeInI!(rmw, uCode::UN_DCP),
    /* D4 */ uCodeZPI!(r, 'x', uCode::UN_NOP),
    /* D5 */ uCodeZPI!(r, 'x', uCode::CMP),
    /* D6 */ uCodeZPI!(rmw, uCode::DEC),
    /* D7 */ uCodeZPI!(rmw, uCode::UN_DCP),
/* $d8 */ uCodeAc_Ap!(uCode::CLD),
    /* D9 */ uCodeAbI!(r, 'y', uCode::CMP),
    /* DA */ [uCode::DummyReadNext, uCode::UN_NOP, uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused],
    /* DB */ uCodeAbI!(rmw, uCode::UN_DCP),
    /* DC */ uCodeAbI!(r, 'x', uCode::UN_NOP),
    /* DD */ uCodeAbI!(r, 'x', uCode::CMP),
    /* DE */ uCodeAbI!(rmw, uCode::DEC),
    /* DF */ uCodeAbI!(rmw, uCode::UN_DCP),

    /* E0 */ uCodeI!(uCode::CPX),
    /* E1 */ uCodeIIn!(r, uCode::SBC),
    /* E2 */ uCodeI!(uCode::UN_NOP),
    /* E3 */ uCodeIIn!(rmw, uCode::UN_ISC),
    /* E4 */ uCodeZP!(r, uCode::CPX),
    /* E5 */ uCodeZP!(r, uCode::SBC),
    /* E6 */ uCodeZP!(rmw, uCode::INC),
    /* E7 */ uCodeZP!(rmw, uCode::UN_ISC),
    /* E8 */ [uCode::DummyReadNext, uCode::INX, uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused],
    /* E9 */ uCodeI!(uCode::SBC),
/* $ea */ uCodeAc_Ap!(uCode::NOP),
    /* EB */ uCodeI!(uCode::UN_SBC),
    /* EC */ uCodeAb!(r, uCode::CPX),
    /* ED */ uCodeAb!(r, uCode::SBC),
    /* EE */ uCodeAb!(rmw, uCode::INC),
    /* EF */ uCodeAb!(rmw, uCode::UN_ISC),

    /* F0 */ [uCode::FetchOperand, uCode::BEQ, uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused],
    /* F1 */ uCodeInI!(r, uCode::SBC),
    /* F2 */ [uCode::UN_STP; 9],
    /* F3 */ uCodeInI!(rmw, uCode::UN_ISC),
    /* F4 */ uCodeZPI!(r, 'x', uCode::UN_NOP),
    /* F5 */ uCodeZPI!(r, 'x', uCode::SBC),
    /* F6 */ uCodeZPI!(rmw, uCode::INC),
    /* F7 */ uCodeZPI!(rmw, uCode::UN_ISC),
    /* F8 */ [uCode::DummyReadNext, uCode::SED, uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused],
    /* F9 */ uCodeAbI!(r, 'y', uCode::SBC),
    /* FA */ [uCode::DummyReadNext, uCode::UN_NOP, uCode::FetchOpcode, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused, uCode::Unused],
    /* FB */ uCodeAbI!(rmw, uCode::UN_ISC),
    /* FC */ uCodeAbI!(r, 'x', uCode::UN_NOP),
    /* FD */ uCodeAbI!(r, 'x', uCode::SBC),
    /* FE */ uCodeAbI!(rmw, uCode::INC),
    /* FF */ uCodeAbI!(rmw, uCode::UN_ISC),
];