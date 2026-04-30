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
  ucode_curr: &'static [uCode]
}