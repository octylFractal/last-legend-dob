use std::fmt::{Debug, Formatter};

use binrw::{BinRead, BinWrite};

#[derive(BinRead, BinWrite)]
pub struct U32Size(
    #[br(map = |r: u32| usize::try_from(r).expect("failed to convert u32 to usize"))]
    #[bw(map = |r| u32::try_from(*r).expect("failed to convert usize to u32"))]
    pub usize,
);

impl Debug for U32Size {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Simple function to parse args from a string for using in [Command].
pub(crate) fn parse_command_args(stuff: &str) -> Vec<String> {
    #[derive(Debug, Default)]
    struct Parser {
        parsed: Vec<String>,
        current_str: Vec<char>,
        in_quote: bool,
        backslash: bool,
        used_quote: bool,
    }
    impl Parser {
        fn push_arg(&mut self) {
            let mut str: String = self.current_str.iter().collect();
            self.current_str.clear();
            if !self.used_quote {
                let trimmed = str.trim();
                if trimmed.is_empty() {
                    // This is separating whitespace...
                    return;
                }
                str = trimmed.to_string();
            }
            self.used_quote = false;
            self.parsed.push(str);
        }
    }
    let mut parser = Parser::default();

    for c in stuff.chars() {
        match c {
            '\\' => {
                if parser.backslash {
                    parser.current_str.push('\\');
                }
                parser.backslash = !parser.backslash;
            }
            '"' if !parser.backslash => {
                parser.in_quote = !parser.in_quote;
                if parser.in_quote {
                    parser.used_quote = true;
                }
            }
            ' ' if !parser.in_quote && !parser.backslash => parser.push_arg(),
            _ => parser.current_str.push(c),
        }
    }
    // Push last arg if present
    parser.push_arg();
    parser.parsed
}

#[cfg(test)]
mod test {
    use super::parse_command_args;

    #[test]
    fn parses_args_simple() {
        assert_eq!(["a"].as_slice(), parse_command_args("a").as_slice(),);
        assert_eq!(["a", "b"].as_slice(), parse_command_args("a b").as_slice(),);
        assert_eq!(
            ["abra", "cadabra"].as_slice(),
            parse_command_args("abra cadabra").as_slice(),
        );
        assert_eq!(
            ["f", "r", "c"].as_slice(),
            parse_command_args("f r c").as_slice(),
        );
    }

    #[test]
    fn parses_args_extra_whitespace() {
        assert_eq!(["a"].as_slice(), parse_command_args("a     ").as_slice(),);
        assert_eq!(
            ["a", "b"].as_slice(),
            parse_command_args("a    b").as_slice(),
        );
        assert_eq!(
            ["abra", "cadabra"].as_slice(),
            parse_command_args("abra \t\tcadabra").as_slice(),
        );
        assert_eq!(
            ["f", "r", "c"].as_slice(),
            parse_command_args("\tf\t r\t c\t").as_slice(),
        );
    }
}
