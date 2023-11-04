use crate::Parser;
use alloc::vec::Vec;
use bytes::{BufMut, Bytes, BytesMut};

/// A struct representing a 2 byte IAC sequence.
#[derive(Clone, Copy, Debug)]
pub struct TelnetIAC {
  pub command: u8,
}

impl From<TelnetIAC> for Bytes {
  fn from(val: TelnetIAC) -> Self {
    let mut buf = BytesMut::with_capacity(2);
    buf.put_u8(255);
    buf.put_u8(val.command);
    buf.freeze()
  }
}

impl From<TelnetIAC> for Vec<u8> {
  fn from(val: TelnetIAC) -> Self {
    let b: Bytes = val.into();
    b.to_vec()
  }
}

impl TelnetIAC {
  #[must_use] pub fn new(command: u8) -> Self {
    Self { command }
  }
  /// Consume the sequence struct and return the bytes.
  #[must_use] pub fn into_bytes(self) -> Vec<u8> {
    self.into()
  }
}

/// A struct representing a 3 byte IAC sequence.
#[derive(Clone, Copy, Debug)]
pub struct TelnetNegotiation {
  pub command: u8,
  pub option: u8,
}

impl From<TelnetNegotiation> for Bytes {
  fn from(val: TelnetNegotiation) -> Self {
    let data = [val.command, val.option];
    let mut buf = BytesMut::with_capacity(3);
    buf.put_u8(255);
    buf.put(&data[..]);
    buf.freeze()
  }
}

impl From<TelnetNegotiation> for Vec<u8> {
  fn from(val: TelnetNegotiation) -> Self {
    let b: Bytes = val.into();
    b.to_vec()
  }
}

impl TelnetNegotiation {
  #[must_use] pub fn new(command: u8, option: u8) -> Self {
    Self { command, option }
  }
  /// Consume the sequence struct and return the bytes.
  #[must_use] pub fn into_bytes(self) -> Vec<u8> {
    self.into()
  }
}

/// A struct representing an arbitrary length IAC subnegotiation sequence.
#[derive(Clone, Debug)]
pub struct TelnetSubnegotiation {
  pub option: u8,
  pub buffer: Bytes,
}

impl From<TelnetSubnegotiation> for Bytes {
  fn from(val: TelnetSubnegotiation) -> Self {
    let head: [u8; 3] = [255, 250, val.option];
    let parsed = &Parser::escape_iac(val.buffer)[..];
    let tail: [u8; 2] = [255, 240];
    let mut buf = BytesMut::with_capacity(head.len() + parsed.len() + tail.len());
    buf.put(&head[..]);
    buf.put(parsed);
    buf.put(&tail[..]);
    buf.freeze()
  }
}

impl From<TelnetSubnegotiation> for Vec<u8> {
  fn from(val: TelnetSubnegotiation) -> Self {
    let b: Bytes = val.into();
    b.to_vec()
  }
}

impl TelnetSubnegotiation {
  pub fn new(option: u8, buffer: Bytes) -> Self {
    Self { option, buffer }
  }
  /// Consume the sequence struct and return the bytes.
  pub fn into_bytes(self) -> Vec<u8> {
    self.into()
  }
}

/// An enum representing various telnet events.
#[derive(Clone, Debug)]
pub enum TelnetEvents {
  /// An IAC command sequence.
  IAC(TelnetIAC),
  /// An IAC negotiation sequence.
  Negotiation(TelnetNegotiation),
  /// An IAC subnegotiation sequence.
  Subnegotiation(TelnetSubnegotiation),
  /// Regular data received from the remote end.
  DataReceive(Bytes),
  /// Any data to be sent to the remote end.
  DataSend(Bytes),
  /// MCCP2/3 compatibility. MUST DECOMPRESS THIS DATA BEFORE PARSING
  DecompressImmediate(Bytes),
}

impl From<TelnetEvents> for Bytes {
  fn from(val: TelnetEvents) -> Self {
    match val {
      TelnetEvents::IAC(iac) => iac.into(),
      TelnetEvents::Negotiation(neg) => neg.into(),
      TelnetEvents::Subnegotiation(sub) => sub.into(),
      TelnetEvents::DataReceive(data) => data,
      TelnetEvents::DataSend(data) => data,
      TelnetEvents::DecompressImmediate(data) => data,
    }
  }
}

impl TelnetEvents {
  /// Helper method to generate a `TelnetEvents::DataSend`.
  pub fn build_send(buffer: Bytes) -> Self {
    TelnetEvents::DataSend(buffer)
  }
  /// Helper method to generate a `TelnetEvents::DataReceive`.
  pub fn build_receive(buffer: Bytes) -> Self {
    TelnetEvents::DataReceive(buffer)
  }
  /// Helper method to generate a `TelnetEvents::IAC`.
  #[must_use] pub fn build_iac(command: u8) -> TelnetEvents {
    TelnetEvents::IAC(TelnetIAC::new(command))
  }
  /// Helper method to generate a `TelnetEvents::Negotiation`.
  #[must_use] pub fn build_negotiation(command: u8, option: u8) -> Self {
    TelnetEvents::Negotiation(TelnetNegotiation::new(command, option))
  }
  /// Helper method to generate a `TelnetEvents::Subnegotiation`.
  pub fn build_subnegotiation(option: u8, buffer: Bytes) -> Self {
    TelnetEvents::Subnegotiation(TelnetSubnegotiation::new(option, buffer))
  }
}
