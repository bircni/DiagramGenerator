# Diagram Generator

[![Crates.io](https://img.shields.io/crates/v/diagen.svg)](https://crates.io/crates/diagen)
[![CI](https://github.com/bircni/DiagramGenerator/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/bircni/DiagramGenerator/actions/workflows/ci.yml)

<!-- [![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/bircni/diagen/blob/main/LICENSE) -->

Generate a diagram from your rust code.

## Usage

```text
A tool for generating a diagram for a Rust project based on its source code.

Usage: diagen.exe [OPTIONS]

Options:
  -p, --path <PATH>          Path to main.rs or lib.rs or the root of the crate
  -o, --output <OUTPUT>      Path to output the diagram
  -l, --loglevel <LOGLEVEL>  Log Level Filter [Debug, Info, Error, Warn] [default: Info]
  -n, --name <NAME>          Name of the Diagram [default: Diagram]
  -t, --include-tests        Include test functions in the diagram (excluded by default)
  -h, --help                 Print help
  -V, --version              Print version
```

The generated diagram will be saved in the same directory where you run the command.
