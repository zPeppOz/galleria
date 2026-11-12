use crate::*;
#[derive(Debug, Clone)]
pub struct Rejected {
    pub line: usize,
    pub name: String,
    pub class: Class,
}
pub struct Filtered {
    pub items: Vec<Item>,
    pub warnings: Vec<Rejected>,
}
fn rejection(item: &Item) -> Option<Rejected> {
    let (line, name, class) = match item {
        Item::Directive { line, name, args } => (*line, name, classify(name, args)),
        Item::Block { line, tag, .. } => (
            *line,
            tag,
            if FILE_BLOCKS.contains(&tag.as_str()) {
                Class::Allowed
            } else {
                Class::Unknown
            },
        ),
        _ => return None,
    };
    (class != Class::Allowed).then(|| Rejected {
        line,
        name: name.clone(),
        class,
    })
}
pub fn check_strict(items: &[Item]) -> Result<(), Vec<Rejected>> {
    let errors: Vec<_> = items.iter().filter_map(rejection).collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
pub fn import_filter(items: Vec<Item>) -> Filtered {
    let mut result = Filtered {
        items: Vec::new(),
        warnings: Vec::new(),
    };
    for item in items {
        if let Some(r) = rejection(&item) {
            if r.class != Class::DaemonOwned {
                result.warnings.push(r);
            }
        } else {
            result.items.push(item);
        }
    }
    result
}
pub fn has_directive(items: &[Item], name: &str) -> bool {
    items
        .iter()
        .any(|i| matches!(i,Item::Directive{name:n,..} if n==name))
}
