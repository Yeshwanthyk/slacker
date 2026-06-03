#![doc = "Command-line entrypoint for the slacker emoji converter."]

use std::env;
use std::io::{self, Write};
use std::process::ExitCode;

use slacker::{Error, Product, make, parse};

fn main() -> ExitCode {
    match run() {
        Ok(product) => write_success(&product),
        Err(error) => write_failure(&error),
    }
}

fn run() -> Result<Product, Error> {
    let config = parse(env::args().skip(1))?;
    make(&config)
}

fn write_success(product: &Product) -> ExitCode {
    let mut out = io::stdout().lock();
    let result = if product.json {
        writeln!(
            out,
            "{{\"path\":\"{}\",\"bytes\":{},\"name\":\"{}\"}}",
            product.path.display(),
            product.bytes,
            product.name
        )
    } else {
        writeln!(out, "{}", product.path.display())
    };

    if result.is_err() {
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn write_failure(error: &Error) -> ExitCode {
    let mut err = io::stderr().lock();
    if writeln!(err, "error: {error}").is_err() {
        return ExitCode::FAILURE;
    }
    ExitCode::FAILURE
}
