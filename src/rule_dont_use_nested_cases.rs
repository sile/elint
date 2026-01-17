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
    if v.clauses().count() != 2 {
        return Ok(());
    }

    let mut ok_body = None;
    let mut has_error = false;
    for clause in v.clauses() {
        let p = clause.pattern();
        if ast.is_atom(p, "ok") || ast.is_tagged_tuple(p, "ok") {
            ok_body=Some(clause.body());
        } else if ast.is_atom(p, "error") || ast.is_tagged_tuple(p, "error") {
            has_error = true;
        }
    }

    if let Some(body)=ok_body && has_error {
        todo!()
    }

    Ok(())
}
