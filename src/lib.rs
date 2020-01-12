use std::fs;
use std::path::PathBuf;
use sqlparser::ast;
use sqlparser::dialect;
use sqlparser::parser::Parser;
#[macro_use]
extern crate log;

/// Lint the given set of files for errors and print them to stdout.
/// Returns true if successful, false if errors occurred.
pub fn lint(files: Vec<PathBuf>) -> bool {
    return files.iter().fold(true, |success, file| success && lint_one(file));
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ErrorCode {
    FileError,
    SyntaxError,
    NotNullColumn,
    DefaultValue,
    NonConcurrentIndex,
}

#[derive(Debug, Clone)]
struct LintError {
    code: ErrorCode,
    message: String,
}

impl PartialEq for LintError {
    // Ignore the details of the message for the purpose of comparison.
    fn eq(&self, other: &LintError) -> bool {
        return self.code == other.code;
    }
}

impl LintError {
    /// Create a new error
    pub fn new(code: ErrorCode, message: &str) -> LintError {
        return LintError{code: code, message: message.to_string()};
    }
}

fn lint_one(file: &PathBuf) -> bool {
    debug!("Linting {}...", file.as_path().to_string_lossy());
    let errors = lint_errors(file);
    errors.iter().for_each(|e| {
        println!("{}:{:?}:{}", file.as_path().to_string_lossy(), e.code, e.message);
    });
    errors.is_empty()
}

fn lint_errors(file: &PathBuf) -> Vec<LintError> {
    let contents = match fs::read_to_string(file.as_path()) {
        Err(e) => return vec![LintError::new(ErrorCode::FileError, &e.to_string())],
        Ok(contents) => contents,
    };
    let dialect = dialect::PostgreSqlDialect{};
    let ast = match Parser::parse_sql(&dialect, contents) {
        Err(e) => return vec![LintError::new(ErrorCode::SyntaxError, &e.to_string())],
        Ok(ast) => ast,
    };
    return ast.iter().map(|stmt| lint_statement(stmt)).collect::<Vec<_>>().concat();
}

fn lint_statement(stmt: &ast::Statement) -> Vec<LintError> {
    return match stmt {
        ast::Statement::AlterTable{name: _, operation} => lint_alter_table(operation),
        ast::Statement::CreateIndex{name, concurrently, ..} => lint_create_index(name, *concurrently),
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
            ast::ColumnOption::NotNull => Some(LintError::new(ErrorCode::NotNullColumn, format!(
                "Column {} is added with the NOT NULL option. This can case a full table rewrite which can be very slow.", def.name).as_str())),
            ast::ColumnOption::Default(_) => Some(LintError::new(ErrorCode::DefaultValue, format!(
                "Column {} is added with a default value. This can case a full table rewrite which can be very slow.", def.name).as_str())),
            _ => None,
        }
    }).collect::<Vec<_>>();
}

fn lint_create_index(name: &ast::ObjectName, concurrently: bool) -> Vec<LintError> {
    return if concurrently {
        Vec::new()
    } else {
        vec![LintError::new(ErrorCode::NonConcurrentIndex, format!(
            "Index {} is created without CONCURRENTLY. This requires holding an exclusive table lock while the index is built, which can cause downtime.", name).as_str())]
    }
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
    fn test_lint_add_column_without_default() {
        let errors = lint_errors(&PathBuf::from("test_data/add_column_without_default.sql"));
        assert_eq!(0, errors.len());
    }

    #[test]
    fn test_lint_add_column_with_default() {
        let errors = lint_errors(&PathBuf::from("test_data/add_column_with_default.sql"));
        assert_eq!(vec![LintError::new(ErrorCode::DefaultValue, "")], errors);
    }

    #[test]
    fn test_lint_create_index_sync() {
        let errors = lint_errors(&PathBuf::from("test_data/create_index_sync.sql"));
        assert_eq!(vec![LintError::new(ErrorCode::NonConcurrentIndex, "")], errors);
    }

    #[test]
    fn test_lint_create_index_async() {
        let errors = lint_errors(&PathBuf::from("test_data/create_index_async.sql"));
        assert_eq!(0, errors.len());
    }

}
