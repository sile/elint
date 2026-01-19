use crate::Ast;
use crate::item::{self, ItemKind};

pub const RULE: crate::Rule = crate::Rule::new(
    "nested-cases",
    include_str!("../rules/nested-cases.md"),
    check,
);

pub fn check(ast: &Ast) -> Vec<crate::Span> {
    assert_eq!(ast.root().kind(), ItemKind::Module); //TODO

    let mut errors = Vec::new();
    for item in ast.item_views() {
        let Ok(v) = item::CaseView::new(item) else {
            continue;
        };
        if !check_case(ast, v) {
            errors.push(item.span());
        }
    }
    errors
}

fn check_case(ast: &Ast, v: item::CaseView) -> bool {
    let mut ok_body = None;
    let mut has_error = false;
    for clause in v.clauses() {
        let p = clause.pattern();
        if ast.is_atom(p, "ok") || ast.is_tagged_tuple(p, "ok") {
            if ok_body.is_some() {
                return true;
            }
            ok_body = Some(clause.body());
        } else if ast.is_atom(p, "error") || ast.is_tagged_tuple(p, "error") {
            has_error = true;
        } else {
            return true;
        }
    }

    if let Some(body) = ok_body
        && has_error
    {
        check_nested_case(ast, body)
    } else {
        true
    }
}

fn check_nested_case(ast: &Ast, body: item::ItemsView) -> bool {
    let v = body.last().expect("bug");
    let Ok(v) = item::CaseView::new(v) else {
        return true;
    };

    let mut has_ok = false;
    let mut has_error = false;
    for clause in v.clauses() {
        let p = clause.pattern();
        if ast.is_atom(p, "ok") || ast.is_tagged_tuple(p, "ok") {
            if has_ok {
                return true;
            }
            has_ok = true;
        } else if ast.is_atom(p, "error") || ast.is_tagged_tuple(p, "error") {
            has_error = true;
        } else {
            return true;
        }
    }

    if has_ok && has_error {
        return false;
    }

    true
}
