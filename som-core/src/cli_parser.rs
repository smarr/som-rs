use std::path::PathBuf;

#[derive(Default, Debug)]
pub struct CLIOptions {
    /// File to evaluate.
    pub file: Option<PathBuf>,

    /// Arguments to pass to the `#run:` function.
    pub args: Vec<String>,

    /// Set search path for application classes.
    pub classpath: Vec<PathBuf>,

    /// Disassemble the class, instead of executing.
    pub disassemble: bool,

    /// Enable verbose output (with timing information).
    pub verbose: bool,

    /// Enable verbose output (with timing information).
    pub heap_size: Option<usize>,
}

impl CLIOptions {
    pub fn parse() -> Self {
        // Legacy argument parsing handling: main difference is classpath passed as "-c"
        // Hack, but can be removed if we ever stop caring about comparing to older versions of som-rs
        if std::env::args().any(|x| x == "-c") {
            return Self::parse_som_rs();
        }

        Self::parse_som()
    }

    /// Parse arguments like we do in other SOM VMs.
    /// We can't use clap since we want to take in "-cp", so a naive loop it is
    fn parse_som() -> Self {
        let mut cli_opts = Self::default();

        let mut iter = std::env::args().skip(1);
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "-cp" => {
                    if let Some(full_path) = iter.next() {
                        cli_opts.classpath = full_path.split(":").map(PathBuf::from).collect();
                    } else {
                        panic!("Error: Missing value for -cp");
                    }
                }
                "-disassemble" | "-d" => {
                    cli_opts.disassemble = true;
                }
                "-verbose" | "-v" => {
                    cli_opts.verbose = true;
                }
                "-heapsize" => {
                    if let Some(size_str) = iter.next() {
                        match size_str.parse::<usize>() {
                            Ok(size) => cli_opts.heap_size = Some(size),
                            Err(_) => {
                                panic!("Error: Invalid value for -heap: {}", size_str);
                            }
                        }
                    } else {
                        panic!("Error: Missing value for -heapsize");
                    }
                }
                _ => {
                    if cli_opts.file.is_none() {
                        cli_opts.file = Some(PathBuf::from(arg));
                    } else {
                        cli_opts.args.push(arg);
                    }
                }
            }
        }

        cli_opts
    }

    /// Legacy argument parsing: parse arguments like somrs was original designed to handle.
    /// If we stop supporting this, RebenchDB could not compare old versions of somrs with newer ones.
    /// If we don't support "normal" SOM argument parsing, RebenchDB could not compare somrs with other SOM interpreters since they'd take in different arguments.
    fn parse_som_rs() -> Self {
        let mut cli_opts = Self::default();

        let mut iter = std::env::args().skip(1);
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "-c" => {
                    let mut arg = iter.next();
                    while arg.is_some() && !arg.as_ref().unwrap().starts_with("--") {
                        cli_opts.classpath.push(PathBuf::from(arg.unwrap()));
                        arg = iter.next();
                    }
                }
                "--disassemble" | "-d" => {
                    cli_opts.disassemble = true;
                }
                "--verbose" | "-v" => {
                    cli_opts.verbose = true;
                }
                "--heapsize" => {
                    if let Some(size_str) = iter.next() {
                        match size_str.parse::<usize>() {
                            Ok(size) => cli_opts.heap_size = Some(size),
                            Err(_) => {
                                panic!("Error: Invalid value for -heap: {}", size_str);
                            }
                        }
                    } else {
                        panic!("Error: Missing value for -heapsize");
                    }
                }
                _ => {
                    cli_opts.file = Some(PathBuf::from(arg));
                    for arg in iter.by_ref() {
                        cli_opts.args.push(arg);
                    }
                }
            }
        }

        cli_opts
    }
}
