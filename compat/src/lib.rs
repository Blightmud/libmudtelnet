use libmudtelnet::compatibility::CompatibilityTable;
use libmudtelnet::Parser;

#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct TelnetApplication {
  pub options: Vec<(u8, u8)>,
  pub received_data: Vec<Vec<u8>>,
}

pub fn test_app(app: &TelnetApplication) {
  let mut parser = Parser::with_support(CompatibilityTable::from_options(&app.options));
  for data in &app.received_data {
    parser.receive(data);
  }
}

pub fn test_escape(data: Vec<u8>) {
  let escaped = Parser::escape_iac(data.clone());
  let unescaped = Parser::unescape_iac(escaped);
  assert_eq!(data, unescaped);
}
