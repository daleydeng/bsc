use std::fs;
use std::io::BufReader;
use std::path::Path;

pub(crate) fn validate(path: &Path) -> Result<(), String> {
    let file = fs::File::open(path)
        .map_err(|error| format!("open generated VCD {}: {error}", path.display()))?;
    let mut parser = vcd::Parser::new(BufReader::new(file));
    let header = parser
        .parse_header()
        .map_err(|error| format!("parse VCD header {}: {error}", path.display()))?;
    if count_variables(&header.items) == 0 {
        return Err(format!(
            "generated VCD {} declares no signals",
            path.display()
        ));
    }
    for command in parser {
        command.map_err(|error| format!("parse VCD body {}: {error}", path.display()))?;
    }
    Ok(())
}

fn count_variables(items: &[vcd::ScopeItem]) -> usize {
    items
        .iter()
        .map(|item| match item {
            vcd::ScopeItem::Scope(scope) => count_variables(&scope.items),
            vcd::ScopeItem::Var(_) => 1,
            _ => 0,
        })
        .sum()
}
