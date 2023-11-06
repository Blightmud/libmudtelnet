use libmudtelnet::compatibility::CompatibilityTable;
use libmudtelnet::events::{TelnetEvents, TelnetIAC, TelnetNegotiation, TelnetSubnegotiation};
use libmudtelnet::Parser;

use libtelnet_rs::compatibility::CompatibilityTable as OgCompatibilityTable;
use libtelnet_rs::events::TelnetEvents as OgTelnetEvents;
use libtelnet_rs::Parser as OgParser;

#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct TelnetApplication {
  pub options: Vec<(u8, u8)>,
  pub received_data: Vec<Vec<u8>>,
}

pub fn test_app(app: &TelnetApplication) {
  let mut parser = Parser::with_support(CompatibilityTable::from_options(&app.options));
  let mut og_parser = OgParser::with_support(OgCompatibilityTable::from_options(&app.options));

  for data in &app.received_data {
    let our_events = parser.receive(&data);
    let og_events = events(og_parser.receive(&data));
    assert_eq!(our_events, og_events);
  }

  for i in 0..255 {
    assert_eq!(
      parser.options.get_option(i).into_u8(),
      og_parser.options.get_option(i).into_u8()
    );
  }
}

pub fn events(events: Vec<OgTelnetEvents>) -> Vec<TelnetEvents> {
  events.into_iter().map(event).collect()
}

pub fn event(event: OgTelnetEvents) -> TelnetEvents {
  match event {
    OgTelnetEvents::IAC(iac) => TelnetEvents::IAC(TelnetIAC::new(iac.command)),
    OgTelnetEvents::Negotiation(neg) => {
      TelnetEvents::Negotiation(TelnetNegotiation::new(neg.command, neg.option))
    }
    OgTelnetEvents::Subnegotiation(sub) => {
      TelnetEvents::Subnegotiation(TelnetSubnegotiation::new(sub.option, sub.buffer))
    }
    OgTelnetEvents::DataReceive(data) => TelnetEvents::DataReceive(data),
    OgTelnetEvents::DataSend(data) => TelnetEvents::DataSend(data),
    OgTelnetEvents::DecompressImmediate(data) => TelnetEvents::DecompressImmediate(data),
  }
}
