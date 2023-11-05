#![cfg_attr(not(feature = "std"), no_std)]
#![warn(clippy::pedantic)]
#![allow(
  clippy::module_name_repetitions,
  clippy::fn_params_excessive_bools,
  clippy::struct_excessive_bools
)]

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std as alloc;

use alloc::{format, vec, vec::Vec};

use bytes::{BufMut, Bytes, BytesMut};

pub use bytes;
pub mod compatibility;
pub mod events;
pub mod telnet;

use compatibility::{CompatibilityEntry, CompatibilityTable};
use telnet::op_command::{DO, DONT, EOR, GA, IAC, NOP, SB, SE, WILL, WONT};

enum EventType {
  None(Bytes),
  Iac(Bytes),
  SubNegotiation(Bytes, Option<Bytes>),
  Neg(Bytes),
}

// TODO(@cpu): Stop exporting this macro.
#[macro_export]
/// Macro for calling `Bytes::copy_from_slice()`
macro_rules! vbytes {
  ($slice:expr) => {
    Bytes::copy_from_slice($slice)
  };
}

/// A telnet parser that handles the main parts of the protocol.
pub struct Parser {
  pub options: CompatibilityTable,
  buffer: BytesMut,
}

impl Default for Parser {
  fn default() -> Self {
    Parser::with_capacity(128)
  }
}

impl Parser {
  /// Create a default, empty Parser with an internal buffer capacity of 128 bytes.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// Create an empty parser, setting the initial internal buffer capcity.
  #[must_use]
  pub fn with_capacity(size: usize) -> Self {
    Self::with_support_and_capacity(size, CompatibilityTable::default())
  }

  /// Create a parser, directly supplying a `CompatibilityTable`.
  ///
  /// Uses the default initial buffer capacity of 128 bytes.
  #[must_use]
  pub fn with_support(table: CompatibilityTable) -> Self {
    Self::with_support_and_capacity(128, table)
  }

  /// Create an parser, setting the initial internal buffer capacity and directly supplying a `CompatibilityTable`.
  // TODO(@cpu): 'table' should be first arg to match name.
  #[must_use]
  pub fn with_support_and_capacity(size: usize, table: CompatibilityTable) -> Self {
    Self {
      options: table,
      buffer: BytesMut::with_capacity(size),
    }
  }

  /// Receive bytes into the internal buffer.
  ///
  /// # Arguments
  ///
  /// * `data` - The bytes to be received. This should be sourced from the remote side of a connection.
  ///
  /// # Returns
  ///
  /// `Vec<events::TelnetEvents>` - Any events parsed from the internal buffer with the new bytes.
  ///
  pub fn receive(&mut self, data: &[u8]) -> Vec<events::TelnetEvents> {
    self.buffer.put(data);
    self.process()
  }

  pub fn receive_og(&mut self, data: &[u8]) -> Vec<events::TelnetEvents> {
    self.buffer.put(data);
    self.og_process()
  }

  /// Get whether the remote end supports and is using linemode.
  pub fn linemode_enabled(&mut self) -> bool {
    matches!(
      self.options.get_option(telnet::op_option::LINEMODE),
      CompatibilityEntry {
        remote: true,
        remote_state: true,
        ..
      }
    )
  }

  /// Escape IAC bytes in data that is to be transmitted and treated as a non-IAC sequence.
  ///
  /// # Example
  /// `[255, 1, 6, 2]` -> `[255, 255, 1, 6, 2]`
  pub fn escape_iac<T>(data: T) -> Bytes
  where
    Bytes: From<T>,
  {
    let data = Bytes::from(data);
    let mut res = BytesMut::with_capacity(data.len());
    for byte in data {
      res.put_u8(byte);
      if byte == IAC {
        res.put_u8(IAC);
      }
    }
    res.freeze()
  }

  /// Reverse escaped IAC bytes for non-IAC sequences and data.
  ///
  /// # Example
  /// `[255, 255, 1, 6, 2]` -> `[255, 1, 6, 2]`
  pub fn unescape_iac<T>(data: T) -> Bytes
  where
    Bytes: From<T>,
  {
    let data = Bytes::from(data);
    let mut res = BytesMut::with_capacity(data.len());
    for pair in data.chunks(2) {
      match pair {
        [IAC, IAC] => res.put_u8(IAC),
        _ => res.put(pair),
      }
    }
    res.freeze()
  }

  /// Negotiate an option.
  ///
  /// # Arguments
  ///
  /// `command` - A `u8` representing the telnet command code to be negotiated with. Example: WILL (251), WONT (252), DO (253), DONT (254)
  ///
  /// `option` - A `u8` representing the telnet option code that is being negotiated.
  ///
  /// # Returns
  ///
  /// `events::TelnetEvents::DataSend` - A `DataSend` event to be processed.
  ///
  /// # Usage
  ///
  /// This and other methods meant for sending data to the remote end will generate a `TelnetEvents::Send(DataEvent)` event.
  ///
  /// These Send events contain a buffer that should be sent directly to the remote end, as it will have already been encoded properly.
  pub fn negotiate(&mut self, command: u8, option: u8) -> events::TelnetEvents {
    events::TelnetEvents::build_send(events::TelnetNegotiation::new(command, option).into())
  }

  /// Indicate to the other side that you are able and wanting to utilize an option.
  ///
  /// # Arguments
  ///
  /// `option` - A `u8` representing the telnet option code that you want to enable locally.
  ///
  /// # Returns
  ///
  /// `Option<events::TelnetEvents::DataSend>` - The `DataSend` event to be processed, or None if not supported.
  ///
  /// # Notes
  ///
  /// This method will do nothing if the option is not "supported" locally via the `CompatibilityTable`.
  pub fn _will(&mut self, option: u8) -> Option<events::TelnetEvents> {
    match self.options.get_option(option) {
      mut opt @ CompatibilityEntry {
        local: true,
        local_state: false,
        ..
      } => {
        opt.local_state = true;
        self.options.set_option(option, opt);
        Some(self.negotiate(WILL, option))
      }
      _ => None,
    }
  }

  /// Indicate to the other side that you are not wanting to utilize an option.
  ///
  /// # Arguments
  ///
  /// `option` - A `u8` representing the telnet option code that you want to disable locally.
  ///
  /// # Returns
  ///
  /// `Option<events::TelnetEvents::DataSend>` - A `DataSend` event to be processed, or None if the option is already disabled.
  ///
  pub fn _wont(&mut self, option: u8) -> Option<events::TelnetEvents> {
    match self.options.get_option(option) {
      mut opt @ CompatibilityEntry {
        local_state: true, ..
      } => {
        opt.local_state = false;
        self.options.set_option(option, opt);
        Some(self.negotiate(WONT, option))
      }
      _ => None,
    }
  }

  /// Indicate to the other side that you would like them to utilize an option.
  ///
  /// # Arguments
  ///
  /// `option` - A `u8` representing the telnet option code that you want to enable remotely.
  ///
  /// # Returns
  ///
  /// `Option<events::TelnetEvents::DataSend>` - A `DataSend` event to be processed, or None if the option is not supported or already enabled.
  ///
  /// # Notes
  ///
  /// This method will do nothing if the option is not "supported" remotely via the `CompatibilityTable`.
  pub fn _do(&mut self, option: u8) -> Option<events::TelnetEvents> {
    match self.options.get_option(option) {
      CompatibilityEntry {
        remote: true,
        remote_state: false,
        ..
      } => Some(self.negotiate(DO, option)),
      _ => None,
    }
  }

  /// Indicate to the other side that you would like them to stop utilizing an option.
  ///
  /// # Arguments
  ///
  /// `option` - A `u8` representing the telnet option code that you want to disable remotely.
  ///
  /// # Returns
  ///
  /// `Option<events::TelnetEvents::DataSend>` - A `DataSend` event to be processed, or None if the option is already disabled.
  ///
  pub fn _dont(&mut self, option: u8) -> Option<events::TelnetEvents> {
    match self.options.get_option(option) {
      CompatibilityEntry {
        remote_state: true, ..
      } => Some(self.negotiate(DONT, option)),
      _ => None,
    }
  }

  /// Send a subnegotiation for a locally supported option.
  ///
  /// # Arguments
  ///
  /// `option` - A `u8` representing the telnet option code for the negotiation.
  ///
  /// `data` - A `Bytes` containing the data to be sent in the subnegotiation. This data will have all IAC (255) byte values escaped.
  ///
  /// # Returns
  ///
  /// `Option<events::TelnetEvents::DataSend>` - A `DataSend` event to be processed, or None if the option is not supported or is currently disabled.
  ///
  /// # Notes
  ///
  /// This method will do nothing if the option is not "supported" locally via the `CompatibilityTable`.
  pub fn subnegotiation<T>(&mut self, option: u8, data: T) -> Option<events::TelnetEvents>
  where
    Bytes: From<T>,
  {
    match self.options.get_option(option) {
      CompatibilityEntry {
        local: true,
        local_state: true,
        ..
      } => Some(events::TelnetEvents::build_send(
        events::TelnetSubnegotiation::new(option, Bytes::from(data)).into(),
      )),
      _ => None,
    }
  }

  /// Send a subnegotiation for a locally supported option, using a string instead of raw byte values.
  ///
  /// # Arguments
  ///
  /// `option` - A `u8` representing the telnet option code for the negotiation.
  ///
  /// `text` - A `&str` representing the text to be sent in the subnegotation. This data will have all IAC (255) byte values escaped.
  ///
  /// # Returns
  ///
  /// `Option<events::TelnetEvents::DataSend>` - A `DataSend` event to be processed, or None if the option is not supported or is currently disabled.
  ///
  /// # Notes
  ///
  /// This method will do nothing if the option is not "supported" locally via the `CompatibilityTable`.
  pub fn subnegotiation_text(&mut self, option: u8, text: &str) -> Option<events::TelnetEvents> {
    self.subnegotiation(option, Bytes::copy_from_slice(text.as_bytes()))
  }

  /// Directly send a string, with appended `\r\n`, to the remote end, along with an `IAC (255) GOAHEAD (249)` sequence.
  ///
  /// # Returns
  ///
  /// `events::TelnetEvents::DataSend` - A `DataSend` event to be processed.
  ///
  /// # Notes
  ///
  /// The string will have IAC (255) bytes escaped before being sent.
  pub fn send_text(&mut self, text: &str) -> events::TelnetEvents {
    events::TelnetEvents::build_send(Parser::escape_iac(format!("{text}\r\n")))
  }

  /// Extract sub-buffers from the current buffer
  fn extract_event_data(&mut self) -> Vec<EventType> {
    #[derive(Copy, Clone)]
    enum State {
      Normal,
      Iac,
      Neg,
      Sub,
      SubOpt { opt: u8 },
      SubIac { opt: u8 },
    }

    let mut events = Vec::with_capacity(4);
    let mut iter_state = State::Normal;
    let mut cmd_begin = 0;

    // Empty self.buffer into an immutable Bytes we can process.
    // We'll create views of this buffer to pass to the events using 'buf.slice'.
    // Splitting is O(1) and doesn't copy the data. Freezing is zero-cost. Taking a slice is O(1).
    let buf = self.buffer.split().freeze();
    for (index, &val) in buf.iter().enumerate() {
      (iter_state, cmd_begin) = match (&iter_state, val) {
        (State::Normal, IAC) => {
          if cmd_begin != index {
            events.push(EventType::None(buf.slice(cmd_begin..index)));
          }
          (State::Iac, index)
        }
        (State::Iac, IAC) => (State::Normal, cmd_begin), // Double IAC, ignore,
        (State::Iac, GA | EOR | NOP) => {
          events.push(EventType::Iac(buf.slice(cmd_begin..=index)));
          (State::Normal, index + 1)
        }
        (State::Iac, SB) => (State::Sub, cmd_begin),
        (State::Iac, _) => (State::Neg, cmd_begin), // WILL | WONT | DO | DONT | IS | SEND
        (State::Neg, _) => {
          events.push(EventType::Neg(buf.slice(cmd_begin..=index)));
          (State::Normal, index + 1)
        }
        (State::Sub, opt) => (State::SubOpt { opt }, cmd_begin),
        (State::SubOpt { opt } | State::SubIac { opt }, IAC) => {
          (State::SubIac { opt: *opt }, cmd_begin)
        }
        (State::SubIac { opt }, SE)
          if *opt == telnet::op_option::MCCP2 || *opt == telnet::op_option::MCCP3 =>
        {
          // MCCP2/MCCP3 MUST DECOMPRESS DATA AFTER THIS!
          events.push(EventType::SubNegotiation(
            buf.slice(cmd_begin..=index),
            Some(buf.slice(index + 1..)),
          ));
          cmd_begin = buf.len();
          break;
        }
        (State::SubIac { .. }, SE) => {
          events.push(EventType::SubNegotiation(
            buf.slice(cmd_begin..=index),
            None,
          ));
          (State::Normal, index + 1)
        }
        (State::SubIac { opt }, _) => (State::SubOpt { opt: *opt }, cmd_begin),
        (cur_state, _) => (*cur_state, cmd_begin),
      };
    }

    if cmd_begin < buf.len() {
      match iter_state {
        State::Sub | State::SubOpt { .. } | State::SubIac { .. } => {
          events.push(EventType::SubNegotiation(buf.slice(cmd_begin..), None));
        }
        _ => events.push(EventType::None(buf.slice(cmd_begin..))),
      }
    }

    events
  }

  /// The internal parser method that takes the current buffer and generates the corresponding events.
  fn process(&mut self) -> Vec<events::TelnetEvents> {
    let mut event_list = Vec::with_capacity(2);
    for event in self.extract_event_data() {
      match event {
        EventType::None(buffer) | EventType::Iac(buffer) | EventType::Neg(buffer) => {
          match (buffer.first(), buffer.get(1), buffer.get(2)) {
            (Some(&IAC), Some(command), None) if *command != SE => {
              // IAC command
              event_list.push(events::TelnetEvents::build_iac(*command));
            }
            (Some(&IAC), Some(command), Some(opt)) => {
              // Negotiation command
              event_list.extend(self.process_negotiation(*command, *opt));
            }
            (Some(c), _, _) if *c != IAC => {
              // Not an iac sequence, it's data!
              event_list.push(events::TelnetEvents::build_receive(buffer));
            }
            _ => {}
          }
        }
        EventType::SubNegotiation(buffer, remaining) => {
          let len = buffer.len();
          if buffer[len - 2] == IAC && buffer[len - 1] == SE {
            // Valid ending
            let opt = self.options.get_option(buffer[2]);
            if opt.local && opt.local_state && len - 2 >= 3 {
              event_list.push(events::TelnetEvents::build_subnegotiation(
                buffer[2],
                vbytes!(&buffer[3..len - 2]),
              ));
              if let Some(rbuf) = remaining {
                event_list.push(events::TelnetEvents::DecompressImmediate(rbuf));
              }
            }
          } else {
            // Missing the rest
            self.buffer.put(&buffer[..]);
          }
        }
      }
    }
    event_list
  }

  fn process_negotiation(&mut self, command: u8, opt: u8) -> Vec<events::TelnetEvents> {
    let mut entry = self.options.get_option(opt);
    let event = events::TelnetNegotiation::new(command, opt);
    match (command, entry) {
      (
        WILL,
        CompatibilityEntry {
          remote: true,
          remote_state: false,
          ..
        },
      ) => {
        entry.remote_state = true;
        self.options.set_option(opt, entry);
        vec![
          events::TelnetEvents::build_send(vbytes!(&[IAC, DO, opt])),
          events::TelnetEvents::Negotiation(event),
        ]
      }
      (WILL, CompatibilityEntry { remote: false, .. }) => {
        vec![events::TelnetEvents::build_send(vbytes!(&[IAC, DONT, opt]))]
      }
      (
        WONT,
        CompatibilityEntry {
          remote_state: true, ..
        },
      ) => {
        entry.remote_state = false;
        self.options.set_option(opt, entry);
        vec![
          events::TelnetEvents::build_send(vbytes!(&[IAC, DONT, opt])),
          events::TelnetEvents::Negotiation(event),
        ]
      }
      (
        DO,
        CompatibilityEntry {
          local: true,
          local_state: false,
          ..
        },
      ) => {
        entry.local_state = true;
        entry.remote_state = true;
        self.options.set_option(opt, entry);
        vec![
          events::TelnetEvents::build_send(vbytes!(&[IAC, WILL, opt])),
          events::TelnetEvents::Negotiation(event),
        ]
      }
      (
        DO,
        CompatibilityEntry {
          local_state: false, ..
        }
        | CompatibilityEntry { local: false, .. },
      ) => {
        vec![events::TelnetEvents::build_send(vbytes!(&[IAC, WONT, opt]))]
      }
      (
        DONT,
        CompatibilityEntry {
          local_state: true, ..
        },
      ) => {
        entry.local_state = false;
        self.options.set_option(opt, entry);
        vec![
          events::TelnetEvents::build_send(vbytes!(&[IAC, WONT, opt])),
          events::TelnetEvents::Negotiation(event),
        ]
      }
      (DONT | WONT, CompatibilityEntry { .. }) => {
        vec![events::TelnetEvents::Negotiation(event)]
      }
      _ => Vec::default(),
    }
  }

  // TODO(@cpu): Remove soon - hack for testing against original parser.
  fn og_extract_event_data(&mut self) -> Vec<EventType> {
    enum State {
      Normal,
      Iac,
      Neg,
      Sub,
    }

    let mut iter_state = State::Normal;
    let mut events = Vec::with_capacity(4);
    let mut cmd_begin = 0;

    for (index, &val) in self.buffer.iter().enumerate() {
      match iter_state {
        State::Normal => {
          if val == IAC {
            if cmd_begin < index {
              events.push(EventType::None(vbytes!(&self.buffer[cmd_begin..index])));
            }
            cmd_begin = index;
            iter_state = State::Iac;
          }
        }
        State::Iac => {
          match val {
            IAC => iter_state = State::Normal, // Double IAC, ignore
            GA | EOR | NOP => {
              events.push(EventType::Iac(vbytes!(&self.buffer[cmd_begin..=index])));
              cmd_begin = index + 1;
              iter_state = State::Normal;
            }
            SB => iter_state = State::Sub,
            _ => iter_state = State::Neg, // WILL | WONT | DO | DONT | IS | SEND
          }
        }
        State::Neg => {
          events.push(EventType::Neg(vbytes!(&self.buffer[cmd_begin..=index])));
          cmd_begin = index + 1;
          iter_state = State::Normal;
        }
        State::Sub => {
          // Every sub negotiation should be of the form:
          //   IAC SB <option> <optional data> IAC SE
          // Meaning it must:
          //  * Be at least 5 bytes long.
          //  * Start with IAC SB
          //  * End with IAC SE
          let long_enough = index - cmd_begin >= 4;
          let has_prefix = self.buffer[cmd_begin] == IAC && self.buffer[cmd_begin + 1] == SB;
          let has_suffix = val == SE && self.buffer[index - 1] == IAC;
          if long_enough && has_prefix && has_suffix {
            let opt = self.buffer[cmd_begin + 2];
            if opt == telnet::op_option::MCCP2 || opt == telnet::op_option::MCCP3 {
              // MCCP2/MCCP3 MUST DECOMPRESS DATA AFTER THIS!
              events.push(EventType::SubNegotiation(
                vbytes!(&self.buffer[cmd_begin..=index]),
                Some(vbytes!(&self.buffer[index + 1..])),
              ));
              cmd_begin = self.buffer.len();
              break;
            }
            events.push(EventType::SubNegotiation(
              vbytes!(&self.buffer[cmd_begin..=index]),
              None,
            ));
            cmd_begin = index + 1;
            iter_state = State::Normal;
          }
        }
      }
    }
    if cmd_begin < self.buffer.len() {
      match iter_state {
        State::Sub => events.push(EventType::SubNegotiation(
          vbytes!(&self.buffer[cmd_begin..]),
          None,
        )),
        _ => events.push(EventType::None(vbytes!(&self.buffer[cmd_begin..]))),
      }
    }

    // Empty the buffer when we are done
    self.buffer.clear();
    events
  }

  // TODO(@cpu): Remove soon - hack for testing against original parser.
  fn og_process(&mut self) -> Vec<events::TelnetEvents> {
    let mut event_list: Vec<events::TelnetEvents> = Vec::with_capacity(2);
    for event in self.og_extract_event_data() {
      match event {
        EventType::None(buffer) | EventType::Iac(buffer) | EventType::Neg(buffer) => {
          if buffer.is_empty() {
            continue;
          }
          if buffer[0] == IAC {
            match buffer.len() {
              2 => {
                if buffer[1] != SE {
                  // IAC command
                  event_list.push(events::TelnetEvents::build_iac(buffer[1]));
                }
              }
              3 => {
                // Negotiation
                let mut opt = self.options.get_option(buffer[2]);
                let event = events::TelnetNegotiation::new(buffer[1], buffer[2]);
                match buffer[1] {
                  WILL => {
                    if opt.remote && !opt.remote_state {
                      opt.remote_state = true;
                      event_list.push(events::TelnetEvents::build_send(vbytes!(&[
                        IAC, DO, buffer[2]
                      ])));
                      self.options.set_option(buffer[2], opt);
                      event_list.push(events::TelnetEvents::Negotiation(event));
                    } else if !opt.remote {
                      event_list.push(events::TelnetEvents::build_send(vbytes!(&[
                        IAC, DONT, buffer[2]
                      ])));
                    }
                  }
                  WONT => {
                    if opt.remote_state {
                      opt.remote_state = false;
                      self.options.set_option(buffer[2], opt);
                      event_list.push(events::TelnetEvents::build_send(vbytes!(&[
                        IAC, DONT, buffer[2]
                      ])));
                    }
                    event_list.push(events::TelnetEvents::Negotiation(event));
                  }
                  DO => {
                    if opt.local && !opt.local_state {
                      opt.local_state = true;
                      opt.remote_state = true;
                      event_list.push(events::TelnetEvents::build_send(vbytes!(&[
                        IAC, WILL, buffer[2]
                      ])));
                      self.options.set_option(buffer[2], opt);
                      event_list.push(events::TelnetEvents::Negotiation(event));
                    } else if !opt.local {
                      event_list.push(events::TelnetEvents::build_send(vbytes!(&[
                        IAC, WONT, buffer[2]
                      ])));
                    }
                  }
                  DONT => {
                    if opt.local_state {
                      opt.local_state = false;
                      self.options.set_option(buffer[2], opt);
                      event_list.push(events::TelnetEvents::build_send(vbytes!(&[
                        IAC, WONT, buffer[2]
                      ])));
                    }
                    event_list.push(events::TelnetEvents::Negotiation(event));
                  }
                  _ => (),
                }
              }
              _ => (),
            }
          } else {
            // Not an iac sequence, it's data!
            event_list.push(events::TelnetEvents::build_receive(buffer));
          }
        }
        EventType::SubNegotiation(buffer, remaining) => {
          let len = buffer.len();
          if buffer[len - 2] == IAC && buffer[len - 1] == SE {
            // Valid ending
            let opt = self.options.get_option(buffer[2]);
            if opt.local && opt.local_state && len - 2 >= 3 {
              event_list.push(events::TelnetEvents::build_subnegotiation(
                buffer[2],
                vbytes!(&buffer[3..len - 2]),
              ));
              if let Some(rbuf) = remaining {
                event_list.push(events::TelnetEvents::DecompressImmediate(rbuf));
              }
            }
          } else {
            // Missing the rest
            self.buffer.put(&buffer[..]);
          }
        }
      }
    }
    event_list
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use alloc::vec;
  #[derive(Debug)]
  struct TelnetApplication {
    options: Vec<(u8, u8)>,
    received_data: Vec<Vec<u8>>,
  }

  #[test]
  fn test_parser_diff1() {
    test_app(&TelnetApplication {
      options: vec![(255, 254)],
      received_data: vec![vec![255, 255, 255, 255, 255, 254, 255, 0]],
    });
  }

  #[test]
  fn test_parser_diff2() {
    test_app(&TelnetApplication {
      options: vec![],
      received_data: vec![vec![45, 255, 250, 255]],
    });
  }

  #[test]
  fn test_parser_diff3() {
    test_app(&TelnetApplication {
      options: vec![(0, 1)],
      received_data: vec![vec![255, 253, 0]],
    })
  }

  #[test]
  fn test_parser_diff4() {
    test_app(&TelnetApplication {
      options: vec![],
      received_data: vec![vec![255, 250, 255, 255, 240, 250]],
    })
  }

  #[test]
  fn test_parser_diff5() {
    test_app(&TelnetApplication {
      options: vec![],
      received_data: vec![vec![255, 250, 255, 240, 0]],
    })
  }

  #[test]
  fn test_parser_diff6() {
    test_app(&TelnetApplication {
      options: vec![],
      received_data: vec![vec![240, 255, 250, 255, 240, 0]],
    })
  }

  #[test]
  fn test_parser_diff7() {
    test_app(&TelnetApplication {
      options: vec![],
      received_data: vec![vec![255]],
    })
  }

  #[test]
  fn test_parser_diff8() {
    test_app(&TelnetApplication {
      options: vec![],
      received_data: vec![vec![255, 252, 0]],
    })
  }

  #[test]
  fn test_parser_diff9() {
    test_app(&TelnetApplication {
      options: vec![],
      received_data: vec![vec![254, 255, 255, 255, 254, 0]],
    })
  }

  #[test]
  fn test_parser_diff10() {
    test_app(&TelnetApplication {
      options: vec![(255, 254), (1, 0)],
      received_data: vec![vec![255, 253, 255]],
    })
  }

  fn test_app(app: &TelnetApplication) {
    let mut parser = Parser::with_support(CompatibilityTable::from_options(&app.options));
    let mut og_parser = Parser::with_support(CompatibilityTable::from_options(&app.options));

    for data in &app.received_data {
      assert_eq!(parser.receive(&data), og_parser.receive_og(&data));
    }

    assert_eq!(parser.options, og_parser.options);
  }
}
