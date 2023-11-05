#![no_main]

use libfuzzer_sys::arbitrary;
use libfuzzer_sys::arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

use libtelnet_rs::compatibility::CompatibilityTable;
use libtelnet_rs::Parser;

#[derive(Arbitrary, Debug)]
struct TelnetApplication {
  options: Vec<(u8, u8)>,
  received_data: Vec<Vec<u8>>,
}

fuzz_target!(|app: TelnetApplication| {
  let mut parser = Parser::with_support(CompatibilityTable::from_options(&app.options));
  let mut og_parser = Parser::with_support(CompatibilityTable::from_options(&app.options));

  for data in app.received_data {
    assert_eq!(parser.receive(&data), og_parser.receive_og(&data))
  }

  assert_eq!(parser.options, og_parser.options);
});
