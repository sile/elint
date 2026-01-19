use crate::Ast;
use crate::item::ItemKind;

pub const RULE_NAME: &str = "element-bif";
pub const RULE_TEXT: &str = include_str!("../rules/element-bif.md");

pub fn check(ast: &Ast) -> Vec<crate::Error> {
    assert_eq!(ast.root().kind(), ItemKind::Module); //TODO

    let mut errors = Vec::new();
    for item in ast.item_views() {
        let mut children = match item.kind() {
            ItemKind::ModuleFunCall => {
                let mut children = item.children();
                if !children
                    .next()
                    .is_some_and(|t| t.atom_eq(&ast.text, "erlang"))
                {
                    continue;
                }
                children
            }
            ItemKind::FunCall => item.children(),
            _ => continue,
        };

        if !children
            .next()
            .is_some_and(|t| t.atom_eq(&ast.text, "element"))
        {
            continue;
        }
        if !children.next().is_some_and(|t| t.children().count() == 2) {
            continue;
        }

        let message = format!("Lint Rule Details\n=======\n\n{RULE_TEXT}");
        let e = crate::Error::new(item.span(), message);
        errors.push(e);
    }
    errors
}
