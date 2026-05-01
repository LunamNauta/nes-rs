#![allow(dead_code)]

use crate::mos6502::ucode::{uCode, uCodeTable};

pub trait MemoryBus{
  fn write(&mut self, addr: u16, val: u8) -> ();
  fn read(&self, addr: u16) -> u8;
}

bitflags::bitflags! {
  pub struct ProcessorStatus: u8 {
    const Carry     = 1 << 0;
    const Zero      = 1 << 1;
    const Interrupt = 1 << 2;
    const Decimal   = 1 << 3;
    const Break     = 1 << 4;
    const _         = 1 << 5;
    const Overflow  = 1 << 6;
    const Negative  = 1 << 7;
  }
}

pub struct Core<Bus: MemoryBus> {
  a: u8,
  x: u8,
  y: u8,
  sp: u8,
  pc: u16,
  ps: ProcessorStatus,
  bus: Bus,

  addr: u16,
  ptr: u16,
  data: u8,

  sync: bool,
  irq: bool,
  nmi: bool,
  res: bool,
  rdy: bool,
  so: bool,
  rw: bool,

  cycle: u32,
  subcycle: u8,

  ucode_table: &'static uCodeTable,
  ucode_curr: &'static [uCode],
}

impl<Bus: MemoryBus> Core<Bus> {
  pub fn new(bus: Bus, ucode_table: &'static uCodeTable) -> Self {
    Self {
      a: 0,
      x: 0,
      y: 0,
      sp: 0,
      pc: 0,
      ps: ProcessorStatus::empty(),
      bus: bus,

      addr: 0,
      ptr: 0,
      data: 0,

      sync: false,
      irq: false,
      nmi: false,
      res: false,
      rdy: false,
      so: false,
      rw: false,

      cycle: 0,
      subcycle: 0,

      ucode_table: ucode_table,
      ucode_curr: &[]
    }
  }

  fn write_mem(&mut self, addr: u16, val: u8) {
    self.addr = addr;
    self.data = val;
    self.bus.write(addr, val);
  }

  fn read_mem(&mut self, addr: u16) -> u8 {
    self.addr = addr;
    self.data = self.bus.read(addr);
    return self.data;
  }

  fn read_mem_high(&mut self, addr: u16, val: u16) -> u16 {
    let tmp = self.read_mem(addr);
    return (val & 0x00ff) | ((tmp as u16) << 8);
  }

  fn read_mem_low(&mut self, addr: u16, val: u16) -> u16 {
    let tmp = self.read_mem(addr);
    return (val & 0xff00) | ((tmp as u16) << 0);
  }

  fn step_ucode(&mut self) -> Option<&'static uCode> {
    match self.ucode_curr {
      [first, rest @ ..] => {
        self.ucode_curr = rest;
        Some(first)
      }
      [] => None
    }
  }

  pub fn tick(&mut self) {
    let ucode = self.step_ucode();
    if ucode.is_none() { return; }

    match ucode.unwrap() {
      uCode::Unused => {}

      uCode::FetchOpcode => {
        self.ucode_curr = &self.ucode_table[self.read_mem(self.pc) as usize];
        self.pc = self.pc + 1;
      }
      uCode::DummyReadNext_IncPC => {
        self.read_mem(self.pc);
        self.pc = self.pc + 1;
      }
      uCode::DummyReadNext => { self.read_mem(self.pc); }
      uCode::IncPC => { self.pc = self.pc + 1; }
      uCode::FetchValue => { self.read_mem(self.addr); }

      uCode::PushPCH => {
        self.write_mem(self.sp as u16, ((self.pc & 0xff00) >> 8) as u8);
        self.sp = self.sp - 1;
      }
      uCode::PushPCL => {
        self.write_mem(self.sp as u16, ((self.pc & 0x00ff) >> 0) as u8);
        self.sp = self.sp - 1;
      }
      uCode::PushPS => {
        self.write_mem(self.sp as u16, self.ps.bits());
        self.sp = self.sp - 1;
      }
      uCode::PushReg('p') => {
        self.write_mem(self.sp as u16, self.ps.bits());
        self.sp = self.sp - 1;
      }
      uCode::PushReg('a') => {
        self.write_mem(self.sp as u16, self.a);
        self.sp = self.sp - 1;
      }
      uCode::IncSP => { self.sp = self.sp + 1; }
      uCode::PullReg('p') => { self.ps = ProcessorStatus::from_bits(self.read_mem(self.sp as u16)).unwrap(); }
      uCode::PullReg('a') => { self.a = self.read_mem(self.sp as u16); }
      uCode::PullPS => {
        self.ps = ProcessorStatus::from_bits(self.read_mem(self.sp as u16)).unwrap();
        self.sp = self.sp + 1;
      }
      uCode::PullPCL => { self.pc = self.read_mem_low(self.sp as u16, self.pc); }
      uCode::PullPCH => { self.pc = self.read_mem_high(self.sp as u16, self.pc); }
      uCode::StackInternal => { self.sp = self.sp - 1; }
      uCode::CopyPCL_FetchAddrHighPCH => { self.pc = (self.addr & 0x00ff) | ((self.read_mem(self.pc) as u16) << 8); }

      uCode::FetchPCL => { self.pc = self.read_mem_low(0xfffe, self.pc) }
      uCode::FetchPCH => { self.pc = self.read_mem_high(0xffff, self.pc) }

      uCode::FetchAddrLow => {
        self.addr = self.read_mem_low(self.pc, self.addr);
        self.pc = self.pc + 1;
      }
      uCode::FetchAddrHigh => {
        self.addr = self.read_mem_high(self.pc, self.addr);
        self.pc = self.pc + 1;
      }
      uCode::FetchAddrHigh_AddReg('x') => {
        self.addr = self.read_mem_high(self.pc, self.addr);
        self.addr = (self.addr & 0xff00) | (((self.addr & 0x00ff) + self.x as u16) & 0x00ff);
        self.pc = self.pc + 1;       
      }
      uCode::FetchAddrHigh_AddReg('y') => {
        self.addr = self.read_mem_high(self.pc, self.addr);
        self.addr = (self.addr & 0xff00) | (((self.addr & 0x00ff) + self.y as u16) & 0x00ff);
        self.pc = self.pc + 1;       
      }
      uCode::FetchAddr => {
        self.addr = self.read_mem(self.pc) as u16;
        self.pc = self.pc + 1;
      }
    }
  }
}