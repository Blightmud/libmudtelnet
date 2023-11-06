use alloc::vec::Vec;

use bytes::{BufMut, Bytes, BytesMut};

use crate::telnet::op_command::{IAC, SB, SE};
use crate::Parser;

/// A struct representing a 2 byte IAC sequence.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct TelnetIAC {
  pub command: u8,
}

impl TelnetIAC {
  #[must_use]
  pub fn new(command: u8) -> Self {
    Self { command }
  }

  /// Consume the sequence struct and return the bytes.
  #[must_use]
  pub fn to_bytes(self) -> Bytes {
    Bytes::copy_from_slice(&[IAC, self.command])
  }

  #[must_use]
  #[deprecated(since = "0.2.1", note = "Use `to_bytes` instead.")]
  pub fn into_bytes(self) -> Bytes {
    self.to_bytes()
  }
}

/// A struct representing a 3 byte IAC sequence.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct TelnetNegotiation {
  pub command: u8,
  pub option: u8,
}

impl TelnetNegotiation {
  #[must_use]
  pub fn new(command: u8, option: u8) -> Self {
    Self { command, option }
  }

  /// Consume the sequence struct and return the bytes.
  #[must_use]
  pub fn to_bytes(self) -> Bytes {
    Bytes::copy_from_slice(&[IAC, self.command, self.option])
  }

  #[must_use]
  #[deprecated(since = "0.2.1", note = "Use `to_bytes` instead.")]
  pub fn into_bytes(self) -> Bytes {
    self.to_bytes()
  }
}

/// A struct representing an arbitrary length IAC subnegotiation sequence.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TelnetSubnegotiation {
  pub option: u8,
  pub buffer: Bytes,
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for TelnetSubnegotiation {
  fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
    let option = u.arbitrary()?;
    let buffer: Vec<u8> = u.arbitrary()?;
    Ok(Self {
      option,
      buffer: Bytes::from(buffer),
    })
  }
}

impl TelnetSubnegotiation {
  pub fn new(option: u8, buffer: Bytes) -> Self {
    Self { option, buffer }
  }

  #[must_use]
  pub fn to_bytes(self) -> Bytes {
    let head = [IAC, SB, self.option];
    let parsed = &Parser::escape_iac(self.buffer)[..];
    let tail = [IAC, SE];
    let mut buf = BytesMut::with_capacity(head.len() + parsed.len() + tail.len());
    buf.put(&head[..]);
    buf.put(parsed);
    buf.put(&tail[..]);
    buf.freeze()
  }

  #[must_use]
  #[deprecated(since = "0.2.1", note = "Use `to_bytes` instead.")]
  pub fn into_bytes(self) -> Bytes {
    self.to_bytes()
  }
}

/// An enum representing various telnet events.
#[derive(Clone, Debug, Eq, PartialEq)]
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

impl From<TelnetIAC> for TelnetEvents {
  fn from(iac: TelnetIAC) -> Self {
    TelnetEvents::IAC(iac)
  }
}

impl From<TelnetNegotiation> for TelnetEvents {
  fn from(neg: TelnetNegotiation) -> Self {
    TelnetEvents::Negotiation(neg)
  }
}

impl From<TelnetSubnegotiation> for TelnetEvents {
  fn from(sub: TelnetSubnegotiation) -> Self {
    TelnetEvents::Subnegotiation(sub)
  }
}

impl TelnetEvents {
  /// Helper method to generate a `TelnetEvents::DataSend`.
  #[deprecated(since = "0.2.1", note = "Construct enum variant directly or use into.")]
  pub fn build_send(buffer: Bytes) -> Self {
    TelnetEvents::DataSend(buffer)
  }

  /// Helper method to generate a `TelnetEvents::DataReceive`.
  #[deprecated(since = "0.2.1", note = "Construct enum variant directly or use into.")]
  pub fn build_receive(buffer: Bytes) -> Self {
    TelnetEvents::DataReceive(buffer)
  }

  /// Helper method to generate a `TelnetEvents::IAC`.
  #[must_use]
  #[deprecated(since = "0.2.1", note = "Construct enum variant directly or use into.")]
  pub fn build_iac(command: u8) -> TelnetEvents {
    TelnetEvents::IAC(TelnetIAC::new(command))
  }

  /// Helper method to generate a `TelnetEvents::Negotiation`.
  #[must_use]
  #[deprecated(since = "0.2.1", note = "Construct enum variant directly or use into.")]
  pub fn build_negotiation(command: u8, option: u8) -> Self {
    TelnetEvents::Negotiation(TelnetNegotiation::new(command, option))
  }

  /// Helper method to generate a `TelnetEvents::Subnegotiation`.
  #[deprecated(since = "0.2.1", note = "Construct enum variant directly or use into.")]
  pub fn build_subnegotiation(option: u8, buffer: Bytes) -> Self {
    TelnetEvents::Subnegotiation(TelnetSubnegotiation::new(option, buffer))
  }

  #[must_use]
  pub fn to_bytes(self) -> Bytes {
    match self {
      TelnetEvents::IAC(iac) => iac.to_bytes(),
      TelnetEvents::Negotiation(neg) => neg.to_bytes(),
      TelnetEvents::Subnegotiation(sub) => sub.to_bytes(),
      TelnetEvents::DataReceive(data)
      | TelnetEvents::DataSend(data)
      | TelnetEvents::DecompressImmediate(data) => data,
    }
  }
}

/*
TODO(@cpu): remove/retool this stuff in breaking release.
*/
#[allow(clippy::from_over_into)]
impl Into<Bytes> for TelnetIAC {
  fn into(self) -> Bytes {
    self.to_bytes()
  }
}

#[allow(clippy::from_over_into)]
impl Into<Vec<u8>> for TelnetIAC {
  fn into(self) -> Vec<u8> {
    self.to_bytes().into()
  }
}

#[allow(clippy::from_over_into)]
impl Into<Bytes> for TelnetNegotiation {
  fn into(self) -> Bytes {
    self.to_bytes()
  }
}

#[allow(clippy::from_over_into)]
impl Into<Vec<u8>> for TelnetNegotiation {
  fn into(self) -> Vec<u8> {
    self.to_bytes().into()
  }
}

#[allow(clippy::from_over_into)]
impl Into<Bytes> for TelnetSubnegotiation {
  fn into(self) -> Bytes {
    self.to_bytes()
  }
}

#[allow(clippy::from_over_into)]
impl Into<Vec<u8>> for TelnetSubnegotiation {
  fn into(self) -> Vec<u8> {
    self.to_bytes().into()
  }
}
