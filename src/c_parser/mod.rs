//! This module parses C Code.

use nom::{
    self,
    multispace
};
use self::basic::identifier;
use std::{
    fmt,
    fmt::Write,
    str::from_utf8
};
use super::Config;

mod basic;

/// Transforms a file from the source form to its final form.
pub fn transform_file(content: &[u8], config: &Config) -> Result<String, String> {
    let mut functions = Vec::new();
    let mut input = &content[..];
    let mut output = String::new();

    loop {
        match function(&input) {
            Ok((new_input, result)) => {
                input = new_input;
                functions.push(result);
            },
            Err(nom::Err::Incomplete(_)) => break,
            Err(e) => {
                Err(format!("Parser error: {:?}", e))?;
            }
        }
    }

    output.push_str("#define _GNU_SOURCE\n");
    output.push_str("#include<dlfcn.h>\n");
    output.push_str("#include<stdio.h>\n");

    for function in functions {
        output.push_str("\n");
        function
            .get_definition(&mut output, &config.debug_output)
            .map_err(|err| format!("Error writing tranformed file: {}", err))?;
    }

    Ok(output)
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

impl fmt::Display for TypeQualifier {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TypeQualifier::Const => write!(f, "const"),
            TypeQualifier::Restrict => write!(f, "restrict"),
            TypeQualifier::Volatile => write!(f, "volatile"),
            TypeQualifier::Atomic => write!(f, "_Atomic")
        }
    }
}

/// Represents a type in C.
/// 
/// # Note
/// This does not yet correspond to the C standard and just supports a subset of possible types.
#[derive(Debug)]
struct Type {
    /// The qualifiers used in this type.
    qualifiers: Vec<TypeQualifier>,
    /// The specifier used for this type.
    specifier: String,
    /// The amount of pointer indirections on this type.
    pointer: usize
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for qualifier in &self.qualifiers {
            write!(f, "{} ", qualifier)?;
        }

        write!(f, "{}", self.specifier)?;

        if self.pointer > 0 {
            write!(f, " ")?;
        }

        for _ in 0..self.pointer {
            write!(f, "*")?;
        }

        Ok(())
    }
}

impl Type {
    /// Returns a format specifier for this type.
    fn get_format_specifier(&self) -> &'static str {
        match (&self.specifier[..], self.pointer) {
            ("char", 1) => "\\\"%s\\\"",
            ("int", 0) => "%d",
            ("size_t", 0) => "%zd",
            (_, 0) => "{Unknown Type: %d}",
            (_, _) => "%p"
        }
    }

    /// Returns true, if this type is the void type.
    fn is_void(&self) -> bool {
        match (&self.specifier[..], self.pointer) {
            ("void", 0) => true,
            _ => false
        }
    }
}

/// Represents a C function.
#[derive(Debug)]
struct Function {
    /// The return type of the function.
    return_type: Type,
    /// The name identifying the function.
    name: String,
    /// The parameters of the function.
    parameters: Vec<(Type, String)>
}

impl Function {
    /// Writes the signature of this function. Optionally as a function pointer.
    fn get_signature(&self, f: &mut Write, as_pointer: bool) -> fmt::Result {
        if self.return_type.pointer > 0 {
            write!(f, "{}", self.return_type)?;
        } else {
            write!(f, "{} ", self.return_type)?;
        }

        if as_pointer {
            write!(f, "(*original_{})(", self.name)?;
        } else {
            write!(f, "{}(", self.name)?;
        }

        for (i, parameter) in self.parameters.iter().enumerate() {
            write!(f, "{}", parameter.0)?;

            if parameter.0.pointer == 0 {
                write!(f, " ")?;
            }

            write!(f, "{}",  parameter.1)?;

            if i != self.parameters.len() - 1 {
                write!(f, ", ")?;
            }
        }

        write!(f, ")")
    }

    /// Writes the definition of this function.
    fn get_definition(&self, f: &mut Write, output: &str) -> fmt::Result {
        let keep_result = !self.return_type.is_void();

        self.get_signature(f, false)?;
        write!(f, " {{\n")?;

        if output == "-" {
            write!(f, "    FILE *output = stderr;\n")?;
        } else {
            write!(f, "    FILE *output = fopen(\"{}\", \"a\");\n", output)?;
        }

        write!(f, "    ")?;
        self.get_signature(f, true)?;
        write!(f, " = dlsym(RTLD_NEXT, \"{}\");\n", self.name)?;

        write!(f, "    ")?;

        if keep_result {
            if self.return_type.pointer > 0 {
                write!(f, "{}result = ", self.return_type)?;
            } else {
                write!(f, "{} result = ", self.return_type)?;
            }
        }

        write!(f, "original_{}(", self.name)?;

        for (i, parameter) in self.parameters.iter().enumerate() {
            write!(f, "{}", parameter.1)?;

            if i != self.parameters.len() - 1 {
                write!(f, ", ")?;
            }
        }

        write!(f, ");\n")?;

        write!(f, "    fprintf(output, \"")?;

        if keep_result {
            write!(f, "{} = ", self.return_type.get_format_specifier())?;
        }
        write!(f, "{}(", self.name)?;

        for (i, parameter) in self.parameters.iter().enumerate() {
            write!(f, "{}", parameter.0.get_format_specifier())?;

            if i != self.parameters.len() - 1 {
                write!(f, ", ")?;
            }
        }

        write!(f, ")\\n\"")?;

        if keep_result {
            write!(f, ", result")?;
        }

        for parameter in &self.parameters {
            write!(f, ", {}", parameter.1)?;
        }

        write!(f, ");\n")?;

        if output != "-" {
            write!(f, "    fclose(output);\n")?;
        }

        if keep_result {
            write!(f, "    return result;\n")?;
        }

        write!(f, "}}\n")
    }
}

/// Parses a C function.
/// 
/// # Note
/// This does not yet correspond to the C standard and just supports a subset of possible functions.
named!(function<&[u8], Function>,
    do_parse!(
        return_type: parse_type >>
        opt!(multispace) >>
        name: map!(
            identifier,
            |ident| from_utf8(ident).unwrap().to_string()
        ) >>
        opt!(multispace) >>
        parameters: delimited!(
            char!('('),
            ws!(
                separated_list!(
                    ws!(
                        tag!(",")
                    ),
                    pair!(
                        parse_type,
                        map!(
                            identifier,
                            |ident| from_utf8(ident).unwrap().to_string()
                        )
                    )
                )
            ),
            char!(')')
        ) >>
        opt!(multispace) >>
        char!(';') >>
        (Function {
            return_type,
            name,
            parameters
        })
    )
);

/// Parses a C type.
/// 
/// # Note
/// This does not yet correspond to the C standard and just supports a subset of possible types.
named!(parse_type<&[u8], Type>,
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
                    tag!("void") |
                    tag!("char") |
                    tag!("short") |
                    tag!("int") |
                    tag!("long") |
                    tag!("float") |
                    tag!("double") |
                    tag!("signed") |
                    tag!("unsigned") |
                    tag!("_Bool") |
                    tag!("_Complex") |
                    tag!("size_t")
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
    |(qualifiers, specifier, pointer)| {
            Type {
                qualifiers,
                specifier: from_utf8(specifier).expect("nom bug").to_string(),
                pointer: pointer.len()
            }
        }
    )
);