use std::fmt;
use std::fs;
use std::path::PathBuf;
use sqlparser::ast;
use sqlparser::dialect;
use sqlparser::parser::Parser;
#[macro_use]
extern crate log;

pub fn lint(files: Vec<PathBuf>) -> bool {
    return files.iter().fold(true, |success, file| success && lint_one(file));
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ErrorCode {
    FileError = 1,
    SyntaxError = 2,
    NotNullColumn = 3,
    DefaultValue = 4,
    NonConcurrentIndex = 5,
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Debug, Clone)]
struct LintError {
    code: ErrorCode,
    line: u32,
    col: u32,
    message: String,
}

impl PartialEq for LintError {
    // Ignore the details of the message for the purpose of comparison.
    fn eq(&self, other: &LintError) -> bool {
        return self.code == other.code && self.line == other.line && self.col == other.col;
    }
}

fn err(code: ErrorCode, line: u32, col: u32, message: &str) -> LintError {
    return LintError{code: code, line: line, col: col, message: message.to_string()};
}

fn lint_one(file: &PathBuf) -> bool {
    debug!("Linting {}...", file.as_path().to_string_lossy());
    return lint_errors(file).iter().fold(true, |_, e| {
        println!("{}:{}:{}:E{}:{}", file.as_path().to_string_lossy(), e.line, e.col, e.code, e.message);
        false
    })
}

fn lint_errors(file: &PathBuf) -> Vec<LintError> {
    let contents = match fs::read_to_string(file.as_path()) {
        Err(e) => return vec![LintError{code: ErrorCode::FileError, line: 0, col: 0, message: e.to_string()}],
        Ok(contents) => contents,
    };
    let dialect = dialect::PostgreSqlDialect{};
    let ast = match Parser::parse_sql(&dialect, contents) {
        Err(e) => return vec![LintError{code: ErrorCode::SyntaxError, line: 1, col: 1, message: e.to_string()}],
        Ok(ast) => ast,
    };
    return ast.iter().map(|stmt| lint_statement(stmt)).collect::<Vec<_>>().concat();
}

fn lint_statement(stmt: &ast::Statement) -> Vec<LintError> {
    return match stmt {
        ast::Statement::AlterTable{name: _, operation} => lint_alter_table(operation),
        _ => Vec::new(),
    };
}

fn lint_alter_table(operation: &ast::AlterTableOperation) -> Vec<LintError> {
    return match operation {
        ast::AlterTableOperation::AddColumn(def) => lint_add_column(def),
        _ => Vec::new(),
    };
}

fn lint_add_column(def: &ast::ColumnDef) -> Vec<LintError> {
    return def.options.iter().filter_map(|opt| {
        match opt.option {
            ast::ColumnOption::NotNull => Some(err(ErrorCode::NotNullColumn, 1, 1, format!(
                "Column {} is added with the NOT NULL option. This can case a full table rewrite which can be very slow.", def.name).as_str())),
            ast::ColumnOption::Default(_) => Some(err(ErrorCode::NotNullColumn, 1, 1, format!(
                "Column {} is added with a default value. This can case a full table rewrite which can be very slow.", def.name).as_str())),
            _ => None,
        }
    }).collect::<Vec<_>>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_table() {
        let errors = lint_errors(&PathBuf::from("test_data/create_table.sql"));
        assert_eq!(0, errors.len());
    }

    #[test]
    fn test_lint_add_column_with_default() {
        let errors = lint_errors(&PathBuf::from("test_data/add_column_with_default.sql"));
        assert_eq!(vec![err(ErrorCode::DefaultValue, 2, 1, "")], errors);
    }

}
