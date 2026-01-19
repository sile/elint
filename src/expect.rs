#[derive(Debug)]
pub struct Expect {
    rules: std::collections::HashMap<&'static str, Vec<crate::item::Span>>,
}

impl Expect {
    pub fn new(parser: &crate::parse::Parser) -> Result<Self, crate::Error> {
        todo!()
    }
}
