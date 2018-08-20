//! ldpsc creates stuc to preload as shared libraries.

#[macro_use]
extern crate clap;
#[macro_use]
extern crate nom;
extern crate tempfile;

mod c_parser;

use clap::{Arg, App};
use std::{
    fs::File,
    io::{Read,
        self,
        stdin,
        stdout,
        Write
    },
    path::Path,
    process::Command
};
use tempfile::Builder;

/// The main function for this application.
fn main() -> Result<(), String> {
    // Get the configuration.
    let config = get_config();

    // Read and transform the file.
    let file_content = read_file(&config.input_file)
        .map_err(|err| format!("{}: {}", config.input_file, err))?;
    let transformed_content = c_parser::transform_file(&file_content, &config)?;

    // Output the C code if necessary.
    if config.output_to_c {
        write_file(&config.output_file, transformed_content.as_bytes())
            .map_err(|err| format!("{}: {}", config.output_file, err))?;
        return Ok(());
    }

    // Create a temporary directory.
    let tmp_dir = Builder::new().prefix("ldpsc").tempdir()
        .map_err(|err| format!("Error creating temp directory: {}", err))?;

    // Write the C file in the temporary directory.
    let mut output_path = tmp_dir.path().to_path_buf();
    output_path.push("output.c");
    write_file(output_path.to_str().unwrap(), transformed_content.as_bytes())
        .map_err(|err| format!("{:?}: {}", output_path, err))?;
    
    // Run the C compiler.
    let so_path = run_cc(&config, tmp_dir.path(), &output_path)?;

    // Copy the shared object if necessary.
    if config.create_shared_object {
        write_file(&config.output_file, &read_file(&so_path)
                .map_err(|err| format!("{}: {}", &so_path, err))?)
            .map_err(|err| format!("{}: {}", config.output_file, err))?;
        return Ok(());
    }

    // Run the command.
    run_command(&config, &so_path)
}

/// Runs the C compiler on the given file.
fn run_cc(config: &Config, tmp_dir: &Path, output_path: &Path) -> Result<String, String> {
    let mut command = Command::new(&config.c_compiler);

    let mut so_path = tmp_dir.to_path_buf();
    so_path.push("output.so");

    command
        .arg(output_path)
        .arg("-o")
        .arg(&so_path)
        .arg("-shared")
        .arg("-fPIC")
        .arg("-ldl");

    let output = command
        .output()
        .map_err(|err| format!("Running {:?} failed: {}", command, err))?;

    if !output.status.success() {
        Err(format!("{:?} failed", command))?;
    }

    Ok(so_path.to_str().expect("Path could not be converted to string.").to_string())
}

/// Runs the given command preloading the given library.
fn run_command(config: &Config, preload_path: &str) -> Result<(), String> {
    if let Some(args) = &config.command {
        if args.len() == 0 {
            return Err("No command to run found.".to_string());
        }

        let mut command = Command::new(&args[0]);

        for arg in args.iter().skip(1) {
            command.arg(&arg);
        }

        command
            .env("LD_PRELOAD", preload_path);

        let status = command
            .status()
            .map_err(|err| format!("Running {:?} failed: {}", command, err))?;
        
        if !status.success() {
            if let Some(exit_code) = status.code() {
                Err(format!("{:?} finished unsuccessfully with exit code {}", command, exit_code))
            } else {
                Err(format!("{:?} finished unsuccessfully", command))
            }
        } else {
            Ok(())
        }
    } else {
        Err("No command to run found.".to_string())
    }
}

/// This function reads all of the contents of the given file.
fn read_file(file: &str) -> io::Result<Vec<u8>> {
    let mut content = vec![];

    if file == "-" {
        stdin().read_to_end(&mut content)?;
    } else {
        File::open(file)?.read_to_end(&mut content)?;
    }

    Ok(content)
}

/// This function writes the output to the given file.
fn write_file(file: &str, output: &[u8]) -> io::Result<()> {
    if file == "-" {
        stdout().write_all(output)?;
    } else {
        File::create(file)?.write_all(output)?;
    }

    Ok(())
}

/// Represents a configuration for the program.
#[derive(Debug)]
pub struct Config {
    /// The file to read the input from. - for stdin.
    input_file: String,
    /// The file to write the output to. - for stdout.
    output_file: String,
    /// Whether to stop after changing the C code.
    output_to_c: bool,
    /// The file to use for debug output. - for stderr.
    debug_output: String,
    /// The C compiler to use.
    c_compiler: String,
    /// Whether to stop after creating the shared object file.
    create_shared_object: bool,
    /// The command to run.
    command: Option<Vec<String>>
}

/// Returns a configuration for this program.
fn get_config() -> Config {
    let matches = App::new("ldpsc")
        .version(&crate_version!()[..])
        .author(crate_authors!())
        .about("ldpsc (ld preload stub creator) creates stubs to preload as shared libraries.")
        .arg(Arg::with_name("input")
            .required(false)
            .takes_value(true)
            .short("i")
            .long("input")
            .help("The input file")
            .long_help("Specifies the input file where the stubs are located. By default - is used to read from stdin."))
        .arg(Arg::with_name("output-file")
            .required(false)
            .takes_value(true)
            .short("o")
            .long("output")
            .help("Specifies the name of the output file")
            .long_help("The supplied name will be the name of the output file. By default - is used to write to stdout."))
        .arg(Arg::with_name("output-c")
            .required(false)
            .short("c")
            .long("output-c")
            .help("Output C code")
            .long_help("Instead of compiling the result to a shared object file output the C code."))
        .arg(Arg::with_name("debug-output")
            .required(false)
            .takes_value(true)
            .short("d")
            .long("debug-output")
            .help("The file to use for debug output")
            .long_help("Debug messages of the calls are written to this file. By default - is used to write to stderr."))
        .arg(Arg::with_name("c-compiler")
            .required(false)
            .takes_value(true)
            .conflicts_with("output-c")
            .short("C")
            .long("c-compiler")
            .help("The C compiler to use")
            .long_help("The C compiler to use for the creation of the shared object file. By default cc is used."))
        .arg(Arg::with_name("create-so")
            .required(false)
            .conflicts_with("output-c")
            .short("s")
            .long("create-so")
            .help("Output shared object")
            .long_help("Instead of executing a program with the shared object, stop after creating it."))
        .arg(Arg::with_name("command")
            .required(true)
            .multiple(true)
            .conflicts_with("output-c")
            .conflicts_with("create-so")
            .help("The command to run")
            .long_help("The command to run with the preloaded shared object. Only used when the --output-c and --create-so are not used."))
        .get_matches();

    Config {
        input_file: matches.value_of("input").unwrap_or("-").to_string(),
        output_file: matches.value_of("output-file").unwrap_or("-").to_string(),
        output_to_c: matches.is_present("output-c"),
        debug_output: matches.value_of("debug-output").unwrap_or("-").to_string(),
        c_compiler: matches.value_of("c-compiler").unwrap_or("cc").to_string(),
        create_shared_object: matches.is_present("create-so"),
        command: matches.values_of("command")
            .map(|cmds| cmds
                .map(|cmd| cmd.to_string())
                .collect::<Vec<String>>())
    }
}