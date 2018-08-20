//! This module supplies basic functions to parse C code.
//!
//! The C standard referenced is found
//! [here](http://www.open-std.org/jtc1/sc22/wg14/www/abq/c17_updated_proposed_fdis.pdf).

/// Parses a `nondigit` according to the C standard.
named!(pub nondigit,
    recognize!(
        one_of!("_abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ")
    )
);

/// Parses a `hexadecimal-digit` according to the C standard.
named!(pub hexadecimal_digit,
    recognize!(
        one_of!("0123456789abcdefABCDEF")
    )
);

/// Parses a `digit` according to the C standard.
named!(pub digit,
    recognize!(
        one_of!("0123456789")
    )
);

/// Parses a `hex-quad` according to the C standard.
named!(pub hex_quad,
    recognize!(
        count_fixed!(
            &[u8],
            hexadecimal_digit,
            4
        )
    )
);

/// Parses a `universal-character-name` according to the C standard.
named!(pub universal_character_name,
    alt!(
        recognize!(
            tuple!(
                tag!("\\u"),
                hex_quad
            )
        ) |
        recognize!(
            tuple!(
                tag!("\\U"),
                count_fixed!(
                    &[u8],
                    hex_quad,
                    2
                )
            )
        )
    )
);

/// Parses an `identifier-nondigit` according to the C standard.
named!(pub identifier_nondigit,
    recognize!(
        alt!(
            nondigit |
            universal_character_name
        )
    )
);

/// Parses an `identifier` according to the C standard.
named!(pub identifier,
    recognize!(
        pair!(
            identifier_nondigit,
            many0!(
                alt!(
                    identifier_nondigit |
                    digit
                )
            )
        )
    )
);

#[cfg(test)]
mod tests {
    use nom;
    use super::*;

    #[test]
    fn test_nondigit() {
        assert!(nondigit(b"_") == Ok((&[], &[b'_'])));
        assert!(nondigit(b"a") == Ok((&[], &[b'a'])));
        assert!(nondigit(b"G") == Ok((&[], &[b'G'])));
        assert!(nondigit(b"+") == Err(nom::Err::Error(nom::Context::Code(&[b'+'], nom::ErrorKind::OneOf))));
        assert!(nondigit(b"-") == Err(nom::Err::Error(nom::Context::Code(&[b'-'], nom::ErrorKind::OneOf))));
        assert!(nondigit(b"5") == Err(nom::Err::Error(nom::Context::Code(&[b'5'], nom::ErrorKind::OneOf))));
        assert!(nondigit(b"~") == Err(nom::Err::Error(nom::Context::Code(&[b'~'], nom::ErrorKind::OneOf))));
        assert!(nondigit(b"") == Err(nom::Err::Incomplete(nom::Needed::Size(1))));
    }

    #[test]
    fn test_hexadecimal_digit() {
        assert!(hexadecimal_digit(b"a") == Ok((&[], &[b'a'])));
        assert!(hexadecimal_digit(b"0") == Ok((&[], &[b'0'])));
        assert!(hexadecimal_digit(b"8") == Ok((&[], &[b'8'])));
        assert!(hexadecimal_digit(b"D") == Ok((&[], &[b'D'])));
        assert!(hexadecimal_digit(b"f") == Ok((&[], &[b'f'])));
        assert!(hexadecimal_digit(b"F") == Ok((&[], &[b'F'])));
        assert!(hexadecimal_digit(b"g") == Err(nom::Err::Error(nom::Context::Code(&[b'g'], nom::ErrorKind::OneOf))));
        assert!(hexadecimal_digit(b"~") == Err(nom::Err::Error(nom::Context::Code(&[b'~'], nom::ErrorKind::OneOf))));
        assert!(hexadecimal_digit(b"") == Err(nom::Err::Incomplete(nom::Needed::Size(1))));
    }

    #[test]
    fn test_digit() {
        assert!(digit(b"0") == Ok((&[], &[b'0'])));
        assert!(digit(b"3") == Ok((&[], &[b'3'])));
        assert!(digit(b"5") == Ok((&[], &[b'5'])));
        assert!(digit(b"9") == Ok((&[], &[b'9'])));
        assert!(digit(b"g") == Err(nom::Err::Error(nom::Context::Code(&[b'g'], nom::ErrorKind::OneOf))));
        assert!(digit(b"a") == Err(nom::Err::Error(nom::Context::Code(&[b'a'], nom::ErrorKind::OneOf))));
        assert!(digit(b"A") == Err(nom::Err::Error(nom::Context::Code(&[b'A'], nom::ErrorKind::OneOf))));
        assert!(digit(b"`") == Err(nom::Err::Error(nom::Context::Code(&[b'`'], nom::ErrorKind::OneOf))));
        assert!(digit(b"") == Err(nom::Err::Incomplete(nom::Needed::Size(1))));
    }

    #[test]
    fn test_hex_quad() {
        assert!(hex_quad(b"abcd") == Ok((&[], &[b'a', b'b', b'c', b'd'])));
        assert!(hex_quad(b"f00d") == Ok((&[], &[b'f', b'0', b'0', b'd'])));
        assert!(hex_quad(b"1337") == Ok((&[], &[b'1', b'3', b'3', b'7'])));
        assert!(hex_quad(b"123") == Err(nom::Err::Incomplete(nom::Needed::Size(1))));
        assert!(hex_quad(b"") == Err(nom::Err::Incomplete(nom::Needed::Size(1))));
        assert!(hex_quad(b"123g") == Err(nom::Err::Error(nom::Context::Code(&[b'1', b'2', b'3', b'g'], nom::ErrorKind::Count))));
    }

    #[test]
    fn test_universal_character_name() {
        assert!(universal_character_name(b"\\u1337") == Ok((&[], &[b'\\', b'u', b'1', b'3', b'3', b'7'])));
        assert!(universal_character_name(b"\\u78ba") == Ok((&[], &[b'\\', b'u', b'7', b'8', b'b', b'a'])));
        assert!(universal_character_name(b"\\UffAC1234") == Ok((&[], &[b'\\', b'U', b'f', b'f', b'A', b'C', b'1', b'2', b'3', b'4'])));
        assert!(universal_character_name(b"\\UffAC123") == Err(nom::Err::Incomplete(nom::Needed::Size(1))));
        assert!(universal_character_name(b"a123g") == Err(nom::Err::Error(nom::Context::Code(&[b'a', b'1', b'2', b'3', b'g'], nom::ErrorKind::Alt))));
        assert!(universal_character_name(b"") == Err(nom::Err::Incomplete(nom::Needed::Size(2))));
    }

    #[test]
    fn test_identifier_nondigit() {
        assert!(identifier_nondigit(b"\\u1337") == Ok((&[], &[b'\\', b'u', b'1', b'3', b'3', b'7'])));
        assert!(identifier_nondigit(b"a") == Ok((&[], &[b'a'])));
        assert!(identifier_nondigit(b"_") == Ok((&[], &[b'_'])));
        assert!(identifier_nondigit(b"5") == Err(nom::Err::Error(nom::Context::Code(&[b'5'], nom::ErrorKind::Alt))));
        assert!(identifier_nondigit(b"") == Err(nom::Err::Incomplete(nom::Needed::Size(1))));
    }

    #[test]
    fn test_identifier() {
        assert!(identifier(b"_abc789 ") == Ok((&[b' '], &[b'_', b'a', b'b', b'c', b'7', b'8', b'9'])));
        assert!(identifier(b"a+") == Ok((&[b'+'], &[b'a'])));
        assert!(identifier(b"qr\\u1289 ") == Ok((&[b' '], &[b'q', b'r', b'\\', b'u', b'1', b'2', b'8', b'9'])));
        assert!(identifier(b"5abc") == Err(nom::Err::Error(nom::Context::Code(&[b'5', b'a', b'b', b'c'], nom::ErrorKind::Alt))));
        assert!(identifier(b"") == Err(nom::Err::Incomplete(nom::Needed::Size(1))));
    }
}
