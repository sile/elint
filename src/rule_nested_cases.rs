use crate::item::{self, ItemKind};
use crate::{Ast, CheckResult};

pub const RULE_NAME: &str = "nested-cases";
pub const RULE_TEXT: &str = include_str!("../rules/nested-cases.md");

pub fn check(ast: &Ast) -> Result<(), Vec<crate::Error>> {
    assert_eq!(ast.root().kind(), ItemKind::Module); //TODO

    let mut errors = Vec::new();
    for item in ast.item_views() {
        let Ok(v) = item::CaseView::new(item) else {
            continue;
        };
        if let Err(e) = check_case(ast, v).map_err(|e| e.fix_span(item.span())) {
            errors.push(e);
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn check_case(ast: &Ast, v: item::CaseView) -> CheckResult {
    let mut ok_body = None;
    let mut has_error = false;
    for clause in v.clauses() {
        let p = clause.pattern();
        if ast.is_atom(p, "ok") || ast.is_tagged_tuple(p, "ok") {
            if ok_body.is_some() {
                return Ok(());
            }
            ok_body = Some(clause.body());
        } else if ast.is_atom(p, "error") || ast.is_tagged_tuple(p, "error") {
            has_error = true;
        } else {
            return Ok(());
        }
    }

    if let Some(body) = ok_body
        && has_error
    {
        check_nested_case(ast, body)?;
    }

    Ok(())
}

fn check_nested_case(ast: &Ast, body: item::ItemsView) -> CheckResult {
    let v = body.last().expect("bug");
    let Ok(v) = item::CaseView::new(v) else {
        return Ok(());
    };

    let mut has_ok = false;
    let mut has_error = false;
    for clause in v.clauses() {
        let p = clause.pattern();
        if ast.is_atom(p, "ok") || ast.is_tagged_tuple(p, "ok") {
            if has_ok {
                return Ok(());
            }
            has_ok = true;
        } else if ast.is_atom(p, "error") || ast.is_tagged_tuple(p, "error") {
            has_error = true;
        } else {
            return Ok(());
        }
    }

    if has_ok && has_error {
        let message = format!("Lint Rule Details\n=======\n\n{RULE_TEXT}");
        return Err(crate::Error::new(item::Span::ZERO, message));
    }

    Ok(())
}
