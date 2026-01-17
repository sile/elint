use crate::item::{self, ItemKind};
use crate::{Ast, CheckResult};

pub const RULE_TEXT: &str = include_str!("../rules/rule-dont-use-nested-cases.md");

pub fn check(ast: &Ast) -> CheckResult {
    assert_eq!(ast.root().kind(), ItemKind::Module); //TODO

    for item in ast.item_views() {
        let Ok(v) = item::CaseView::new(item) else {
            continue;
        };
        check_case(ast, v)?;
    }

    Ok(())
}

fn check_case(ast: &Ast, v: item::CaseView) -> CheckResult {
    todo!()
}
