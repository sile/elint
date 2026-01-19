use crate::Ast;
use crate::item::ItemKind;

pub const RULE: crate::Rule = crate::Rule::new(
    "element-bif",
    include_str!("../rules/element-bif.md"),
    check,
);

pub fn check(ast: &Ast) -> Vec<crate::Span> {
    assert_eq!(ast.root().kind(), ItemKind::Module); //TODO

    let mut errors = Vec::new();
    for item in ast.item_views() {
        let mut children = match item.kind() {
            ItemKind::ModuleFunCall => {
                let mut children = item.children();
                if !children.next().is_some_and(|t| t.atom_eq("erlang")) {
                    continue;
                }
                children
            }
            ItemKind::FunCall => item.children(),
            _ => continue,
        };

        if !children.next().is_some_and(|t| t.atom_eq("element")) {
            continue;
        }
        if !children.next().is_some_and(|t| {
            t.children().count() == 2
                && t.children()
                    .next()
                    .is_some_and(|t| t.kind() == ItemKind::Integer)
        }) {
            continue;
        }

        errors.push(item.span());
    }
    errors
}
