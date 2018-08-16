//! ldpsc creates stuc to preload as shared libraries.

#[macro_use]
extern crate clap;
#[macro_use]
extern crate nom;

mod c_parser;

use clap::{Arg, App};
use nom::multispace;
use std::{
    fs::File,
    io::{Read, self, stdin},
    str::from_utf8
};

/// The main function for this application.
fn main() -> Result<(), String> {
    let matches = App::new("ldpsc")
        .version(&crate_version!()[..])
        .about("ldpsc (ld preload stub creator) creates stubs to preload as shared libraries.")
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

    named!(test,
        eof!()
    );

    println!("{:?}", c_functions(&file_content));
    Ok(())
}

/// The open file function either opens stdin as a file or the given file, depending on the
/// argument.
fn open_file(file: &str) -> io::Result<Box<Read>> {
    if file == "-" {
        Ok(Box::new(stdin()))
    } else {
        Ok(Box::new(File::open(file)?))
    }
}

/// Represents a C type qualifier.
#[derive(Debug)]
enum TypeQualifier {
    /// The const type qualifier.
    Const,
    /// The restrict type qualifier.
    Restrict,
    /// The volatile type qualifier.
    Volatile,
    /// The _Atomic type qualifier.
    Atomic
}

/// Represents a type in C.
#[derive(Debug)]
struct Type {
    qualifiers: Vec<TypeQualifier>,
    _type: String,
    pointer: usize
}

#[derive(Debug)]
struct Function {
    return_type: Type,
    name: String,
    parameters: Vec<(Type, String)>
}

named!(c_function<&[u8], Function>,
    map!(
        ws!(
            terminated!(
                tuple!(
                    c_type,
                    map!(
                        c_parser::identifier,
                        |ident| from_utf8(ident).unwrap().to_string()
                    ),
                    delimited!(
                        char!('('),
                        separated_list!(
                            ws!(
                                tag!(",")
                            ),
                            pair!(
                                c_type,
                                map!(
                                    c_parser::identifier,
                                    |ident| from_utf8(ident).unwrap().to_string()
                                )
                            )
                        ),
                        char!(')')
                    )
                ),
                char!(';')
            )
        ),
        |(return_type, name, parameters)| {
            Function {
                return_type,
                name,
                parameters
            }
        }
    )
);

named!(c_functions<&[u8], Vec<Function>>,
    terminated!(
        many0!(
            c_function
        ),
        eof!()
    )
);

/// The c_type function parses a type in the c programming language.
named!(c_type<&[u8], Type>,
    map!(
        tuple!(
            many0!(
                terminated!(
                    alt!(
                        value!(
                            TypeQualifier::Const,
                            tag!("const")
                        ) |
                        value!(
                            TypeQualifier::Restrict,
                            tag!("restrict")
                        ) |
                        value!(
                            TypeQualifier::Volatile,
                            tag!("volatile")
                        ) |
                        value!(
                            TypeQualifier::Atomic,
                            tag!("_Atomic")
                        )
                    ),
                    multispace
                )
            ),
            ws!(
                alt!(
                    tag!("int") |
                    tag!("char") |
                    tag!("void")
                )
            ),
            many0!(
                value!(
                    (),
                    tag!("*")
                )
            )
        )
    ,
    |(qualifiers, _type, pointer)| {
            Type {
                qualifiers,
                _type: from_utf8(_type).expect("nom bug").to_string(),
                pointer: pointer.len()
            }
        }
    )
);
