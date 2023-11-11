use bytes::Bytes;

use libmudtelnet::compatibility::{CompatibilityEntry, CompatibilityTable};
use libmudtelnet::events::{TelnetEvents, TelnetSubnegotiation};
use libmudtelnet::telnet::{op_command as cmd, op_option as opt};
use libmudtelnet::Parser;

/// Test the parser and its general functionality.

#[derive(PartialEq, Debug)]
enum Event {
  Iac,
  Negotiation,
  Subnegotiation,
  Recv,
  Send,
  Decom,
}

macro_rules! events {
    ( $( $x:expr ),* ) => {
        {
            #[allow(unused_mut)]
            let mut temp_ce = CapturedEvents::default();
            $(
                temp_ce.push($x);
            )*
            temp_ce
        }
    };
}

#[derive(Default, Debug)]
struct CapturedEvents {
  events: Vec<Event>,
}

impl CapturedEvents {
  fn push(&mut self, event: Event) {
    self.events.push(event);
  }
}

impl PartialEq for CapturedEvents {
  fn eq(&self, other: &Self) -> bool {
    if self.events.len() == other.events.len() {
      self
        .events
        .iter()
        .zip(other.events.iter())
        .all(|(val1, val2)| val1 == val2)
    } else {
      false
    }
  }
}

fn handle_events(event_list: Vec<TelnetEvents>) -> CapturedEvents {
  let mut events = CapturedEvents::default();
  for event in event_list {
    match event {
      TelnetEvents::IAC(ev) => {
        println!("IAC: {}", ev.command);
        events.push(Event::Iac);
      }
      TelnetEvents::Negotiation(ev) => {
        println!("Negotiation: {} {}", ev.command, ev.option);
        events.push(Event::Negotiation);
      }
      TelnetEvents::Subnegotiation(ev) => {
        println!("Subnegotiation: {} {:?}", ev.option, ev.buffer);
        events.push(Event::Subnegotiation);
      }
      TelnetEvents::DataReceive(buffer) => {
        println!(
          "Receive: {}",
          std::str::from_utf8(&buffer[..]).unwrap_or("Bad utf-8 bytes")
        );
        events.push(Event::Recv);
      }
      TelnetEvents::DataSend(buffer) => {
        println!("Send: {:?}", buffer);
        events.push(Event::Send);
      }
      TelnetEvents::DecompressImmediate(buffer) => {
        println!("DECOMPRESS: {:?}", buffer);
        events.push(Event::Decom);
      }
    };
  }
  events
}

#[test]
fn test_parser() {
  let mut instance: Parser = Parser::new();
  instance.options.support_local(opt::GMCP);
  instance.options.support_local(opt::MCCP2);
  if let Some(ev) = instance._will(opt::GMCP) {
    assert_eq!(handle_events(vec![ev]), events![Event::Send]);
  }
  if let Some(ev) = instance._will(opt::MCCP2) {
    assert_eq!(handle_events(vec![ev]), events![Event::Send]);
  }
  assert_eq!(
    handle_events(instance.receive(&[b"Hello, rust!", &[cmd::IAC, cmd::GA][..]].concat())),
    events![Event::Recv, Event::Iac]
  );
  assert_eq!(
    handle_events(instance.receive(&[cmd::IAC, cmd::DO, opt::GMCP])),
    events![]
  );
  assert_eq!(
    handle_events(instance.receive(&[&[cmd::IAC, cmd::DO, 200][..], b"Some random data"].concat())),
    events![Event::Send, Event::Recv]
  );
  assert_eq!(
    handle_events(instance.receive(
      &TelnetSubnegotiation::new(opt::GMCP, Bytes::copy_from_slice(b"Core.Hello {}")).to_bytes()
    ),),
    events![Event::Subnegotiation]
  );
  assert_eq!(
    handle_events(
      instance.receive(
        &[
          &TelnetSubnegotiation::new(opt::GMCP, Bytes::copy_from_slice(b"Core.Hello {}"))
            .to_bytes()[..],
          b"Random text",
          &[cmd::IAC, cmd::GA][..]
        ]
        .concat()
      ),
    ),
    events![Event::Subnegotiation, Event::Recv, Event::Iac]
  );
  assert_eq!(
    handle_events(
      instance.receive(
        &[
          &TelnetSubnegotiation::new(opt::MCCP2, Bytes::copy_from_slice(b" ")).to_bytes()[..],
          b"This is compressed data",
          &[cmd::IAC, cmd::GA][..]
        ]
        .concat()
      ),
    ),
    events![Event::Subnegotiation, Event::Decom]
  );
  assert_eq!(
    // TODO(@cpu): Can data be made easier to understand at a glance?
    handle_events(instance.receive(&[
      87, 104, 97, 116, 32, 105, 115, 32, 121, 111, 117, 114, 32, 112, 97, 115, 115, 119, 111, 114,
      100, 63, 32, 255, 239, 255, 251, 1
    ])),
    events![Event::Recv, Event::Iac, Event::Send]
  );
}

#[test]
fn test_subneg_separate_receives() {
  let mut instance: Parser = Parser::with_capacity(10);
  instance.options.support_local(opt::GMCP);
  instance._will(opt::GMCP);
  let mut events = instance.receive(
    &[
      &[cmd::IAC, cmd::SB, opt::GMCP][..],
      b"Otion.Data { some: json, data: in, here: ! }",
    ]
    .concat(),
  );
  assert_eq!(handle_events(events), events![]);

  events = instance.receive(b"More.Data { some: json, data: in, here: ! }");
  assert_eq!(handle_events(events), events![]);

  events = instance.receive(
    &[
      &[cmd::IAC, cmd::SE][..],
      &[cmd::IAC, cmd::SB, opt::GMCP][..],
      b"Otion.Data { some: json, data: in, here: ! }",
    ]
    .concat(),
  );
  assert_eq!(handle_events(events), events![Event::Subnegotiation]);

  events = instance.receive(
    &[
      b"More.Data { some: json, data: in, here: ! }",
      &[cmd::IAC, cmd::SE][..],
    ]
    .concat(),
  );
  assert_eq!(handle_events(events), events![Event::Subnegotiation]);
}

// Test that receiving a subnegotiation with embedded UTF-8 content works correctly,
// even when the content includes a SE byte.
#[test]
fn test_subneg_utf8_content() {
  use cmd::{IAC, SB, SE};
  use opt::GMCP;

  // Create a parser that will support GMCP.
  let mut parser = Parser::new();
  parser.options.support_local(GMCP);
  parser._will(GMCP);

  // Construct a GMCP message containing a UTF-8 sequence that happens
  // to include SE (0xF0). This should be permitted as long as the SE isn't
  // preceeded by IAC (0xFF). For our test case we'll use the content
  // '👋' (0xF0, 0x9F, 0x91, 0x8B) - where the leading byte is SE.
  let prefix = &[IAC, SB, GMCP][..];
  let wave_emoji = &[0xF0, 0x9F, 0x91, 0x8B][..];
  let suffix = &[IAC, SE][..];
  let gmcp_msg = [prefix, wave_emoji, suffix].concat();

  // Receive the GMCP message with the parser. This should produce one event.
  let events = parser.receive(&gmcp_msg);
  assert_eq!(events.len(), 1, "only expected one event to be parsed");

  // The event should be a Subnegotiation for the GMCP option, with the correct in-tact
  // buffer contents.
  if let TelnetEvents::Subnegotiation(sub) = events.first().unwrap() {
    assert_eq!(sub.option, 201, "option should be GMCP");
    assert_eq!(
      sub.buffer, wave_emoji,
      "buffer should be equal to the wave emoji"
    );
  } else {
    panic!("missing expected DataReceive event");
  }
}

/// Test escaping IAC bytes in a buffer.
#[test]
fn test_escape() {
  let initial = vec![
    cmd::IAC,
    cmd::SB,
    201,
    cmd::IAC,
    205,
    202,
    cmd::IAC,
    cmd::SE,
  ];
  let expected = Bytes::copy_from_slice(&[
    cmd::IAC,
    cmd::IAC,
    cmd::SB,
    201,
    cmd::IAC,
    cmd::IAC,
    205,
    202,
    cmd::IAC,
    cmd::IAC,
    cmd::SE,
  ]);
  assert_eq!(expected, Parser::escape_iac(initial.clone()));
  assert_eq!(initial, Parser::unescape_iac(expected));
}

/// Test unescaping IAC bytes in a buffer.
#[test]
fn test_unescape() {
  let initial = vec![
    cmd::IAC,
    cmd::IAC,
    cmd::SB,
    201,
    cmd::IAC,
    cmd::IAC,
    205,
    202,
    cmd::IAC,
    cmd::IAC,
    cmd::SE,
  ];
  let expected = Bytes::copy_from_slice(&[
    cmd::IAC,
    cmd::SB,
    201,
    cmd::IAC,
    205,
    202,
    cmd::IAC,
    cmd::SE,
  ]);
  assert_eq!(expected, Parser::unescape_iac(initial.clone()));
  assert_eq!(initial, Parser::escape_iac(expected));
}

#[test]
fn test_escape_roundtrip_bug_one() {
  // The original libtelnet-rs mishandles this input:
  let data = vec![cmd::IAC, cmd::IAC, 228];
  let escaped = Parser::escape_iac(data.clone());
  let unescaped = Parser::unescape_iac(escaped);
  assert_eq!(unescaped, data);
}

#[test]
fn test_escape_roundtrip_bug_two() {
  // The original libtelnet-rs mishandles this input:
  let data = vec![228, cmd::IAC, cmd::IAC];
  let escaped = Parser::escape_iac(data.clone());
  let unescaped = Parser::unescape_iac(escaped);
  assert_eq!(unescaped, data);
}

#[test]
fn test_bad_subneg_dbuffer() {
  // Configure opt 0xFF (IAC) as local supported, and local state enabled.
  let entry = CompatibilityEntry::new(true, false, true, false);
  let opts = CompatibilityTable::from_options(&[(cmd::IAC, entry.into_u8())]);
  // Receive a malformed subnegotiation - this should not panic.
  Parser::with_support(opts).receive(&[cmd::IAC, cmd::SB, cmd::IAC, cmd::SE]);
}

#[test]
fn test_into_bytes() {
  let bytes = libmudtelnet::events::TelnetIAC::new(cmd::IAC).to_bytes();
  assert!(!bytes.is_empty())
}

#[cfg(test)]
mod compat_tests {
  use compat::{test_app, TelnetApplication};

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
}
