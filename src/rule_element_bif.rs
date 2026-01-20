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
                if !children.next_eq(ItemKind::Atom, "erlang") {
                    continue;
                }
                children
            }
            ItemKind::FunCall => item.children(),
            _ => continue,
        };

        if !children.next_eq(ItemKind::Atom, "element") {
            continue;
        }
        if !children
            .next_as_args(2)
            .is_some_and(|mut args| args.next_is(ItemKind::Integer))
        {
            continue;
        }

        errors.push(item.span());
    }
    errors
}
