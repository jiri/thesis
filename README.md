# Assembler

## Obtaining

Make sure you have the nightly version of Rust installed. Running the following snippet should yield similar results:

```console
$ cargo --version
cargo 1.26.0-nightly (311a5eda6 2018-03-26)
```

Then you can just clone the project and run `cargo build`.

```console
$ git clone https://github.com/jiri/thesis-assembler.git
$ cd thesis-assembler
$ cargo build
```

## Usage

Usage is documented in the binary itself. You can obtain it by running with the `--help` flag.

```console
$ cargo run -- --help
    Finished dev [unoptimized + debuginfo] target(s) in 0.0 secs
     Running `target/debug/assembler --help`
assembler 0.1.0

USAGE:
    assembler [FLAGS] [OPTIONS] <FILE>

FLAGS:
    -h, --help       Prints help information
        --stdout     Output the resulting binary to stdout
    -V, --version    Prints version information

OPTIONS:
    -o, --output <OUTPUT>     Path to the output file
    -s, --symfile <FILE>      If set, path where the symfile will be outputted
    -w, --whitelist <FILE>    If set, path to a file containing instruction whitelist

ARGS:
    <FILE>    Path to the source file
```

## Whitelisting

The assembler enables it's users to use only whitelisted instructions if a whitelist file is provided. Whitelist is a
JSON file containing an array of allowed mnemonics, like so:

```json
[ "add", "sub", "inc", "dec" ]
```
