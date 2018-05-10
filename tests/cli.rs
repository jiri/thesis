extern crate assert_cli;

#[cfg(test)]
mod integration {
    use assert_cli;

    #[test]
    fn without_args() {
        assert_cli::Assert::main_binary()
            .fails()
            .stderr().contains("required arguments were not provided")
            .stderr().contains("USAGE")
            .unwrap();
    }

    #[test]
    fn version() {
        assert_cli::Assert::main_binary()
            .with_args(&[ "--help" ])
            .stdout().contains(env!("CARGO_PKG_NAME"))
            .stdout().contains(env!("CARGO_PKG_VERSION"))
            .unwrap();
    }

    #[test]
    fn help() {
        assert_cli::Assert::main_binary()
            .with_args(&[ "--help" ])
            .stdout().contains("USAGE")
            .stdout().contains("FLAGS")
            .stdout().contains("OPTIONS")
            .stdout().contains("ARGS")
            .unwrap();
    }

    #[test]
    fn nonexistent_file() {
        assert_cli::Assert::main_binary()
            .with_args(&[ "nonexistent-file" ])
            .fails()
            .stderr().contains("No such file or directory")
            .unwrap();
    }
}
