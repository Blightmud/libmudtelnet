#![no_main]

use libfuzzer_sys::arbitrary;
use libfuzzer_sys::arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

use libmudtelnet::compatibility::CompatibilityTable;
use libmudtelnet::events::TelnetEvents;
use libmudtelnet::Parser;

#[derive(Arbitrary, Debug)]
struct TelnetApplication {
  options: Vec<(u8, u8)>,
  received_data: Vec<Vec<u8>>,
}

fn test_app(app: &TelnetApplication) {
  let mut parser = Parser::with_support(CompatibilityTable::from_options(&app.options));
  let mut og_parser = libtelnet_rs::Parser::with_support(
    libtelnet_rs::compatibility::CompatibilityTable::from_options(&app.options),
  );

  for data in &app.received_data {
    let events = parser.receive(&data);
    let og_events = og_parser.receive(&data);

    use libtelnet_rs::events::TelnetEvents as og_events;

    assert_eq!(events.len(), og_events.len());
    for (i, event) in events.iter().enumerate() {
      let og_event = &og_events[i];
      match (event, og_event) {
        (TelnetEvents::IAC(iac), og_events::IAC(og_iac)) => {
          assert_eq!(iac.to_bytes(), og_iac.into_bytes());
        }
        (TelnetEvents::Negotiation(neg), og_events::Negotiation(og_neg)) => {
          assert_eq!(neg.to_bytes(), og_neg.into_bytes());
        }
        (TelnetEvents::Subnegotiation(subneg), og_events::Subnegotiation(og_subneg)) => {
          assert_eq!(subneg.clone().to_bytes(), og_subneg.clone().into_bytes());
        }

        (TelnetEvents::DataReceive(dr), og_events::DataReceive(og_dr)) => {
          assert_eq!(dr, og_dr);
        }
        (TelnetEvents::DataSend(ds), og_events::DataSend(og_ds)) => {
          assert_eq!(ds, og_ds);
        }
        (TelnetEvents::DecompressImmediate(di), og_events::DecompressImmediate(og_di)) => {
          assert_eq!(di, og_di);
        }
        _ => panic!("mismatched events: {:?} {:?}", event, og_event),
      }
    }
  }

  for i in 0..255 {
    assert_eq!(
      parser.options.get_option(i).into_u8(),
      og_parser.options.get_option(i).into_u8()
    );
  }
}

fuzz_target!(|app: TelnetApplication| {
  test_app(&app);
});
