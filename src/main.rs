use std::path::PathBuf;
extern crate stderrlog;
extern crate structopt;
use structopt::StructOpt;
use linter;

#[derive(Debug, StructOpt)]
#[structopt(name = "sql-perf-linter", about = "A linter to find potential performance issues in PostgreSQL migrations.")]
struct Opts {
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: usize,
    #[structopt(parse(from_os_str))]
    files: Vec<PathBuf>,
}

fn main() {
    let opts = Opts::from_args();
    stderrlog::new()
        .module(module_path!())
        .verbosity(opts.verbose)
        .init()
        .unwrap();
    let code = if linter::lint(opts.files) {
        0
    } else {
        1
    };
    std::process::exit(code);
}
