#[macro_use]
extern crate clap;
#[macro_use]
extern crate nom;

use clap::{Arg, App};
use std::{
    fs::File,
    io::{Read, self, stdin}
};

fn main() -> Result<(), String> {
    let matches = App::new("ldpsc")
        .version(&crate_version!()[..])
        .about("ldpsc (ld preload stub creator) creates stubs to preload shared libraries.")
        .arg(Arg::with_name("input")
             .required(false)
             .help("The input file")
             .long_help("Specifies the input file where the stubs are located. Use - to read from stdin. If nothing is specified, - is used as the default."))
        .get_matches();

    let input = matches.value_of("input").unwrap_or("-");

    let mut file = open_file(input)
        .map_err(|err| format!("{}: {}", input, err))?;

    let mut file_content = vec![];
    file.read_to_end(&mut file_content)
        .map_err(|err| format!("{}: {}", input, err))?;

    println!("{:?}", c_type(&file_content));
    Ok(())
}

fn open_file(file: &str) -> io::Result<Box<Read>> {
    if file == "-" {
        Ok(Box::new(stdin()))
    } else {
        Ok(Box::new(File::open(file)?))
    }
}

named!(c_type<&[u8], (Option<&[u8]>, &[u8], Option<&[u8]>)>,
        tuple!(
            opt!(
                terminated!(
                    tag!("const"),

                )
            ),
            ws!(
                alt!(
                    tag!("int") |
                    tag!("char")
                )
            ),
            opt!(
                tag!("*")
            )
        )
);
