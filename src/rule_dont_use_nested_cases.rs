use crate::item::{self, ItemKind};
use crate::{Ast, CheckResult};

pub const RULE_TEXT: &str = include_str!("../rules/rule-dont-use-nested-cases.md");

pub fn check(ast: &Ast) -> CheckResult {
    assert_eq!(ast.root().kind(), ItemKind::Module); //TODO
    todo!()
}
