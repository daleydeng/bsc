use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticTclListError {
    offset: usize,
    message: &'static str,
}

impl fmt::Display for StaticTclListError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} at byte {}", self.message, self.offset)
    }
}

impl std::error::Error for StaticTclListError {}

pub fn parse_static_tcl_list(source: &str) -> Result<Vec<String>, StaticTclListError> {
    let continued = normalize_tcl_continuations(source);
    let mut parser = ListParser {
        source: &continued,
        offset: 0,
    };
    let mut words = Vec::new();
    parser.skip_whitespace();
    while !parser.is_finished() {
        words.push(parser.word()?);
        if !parser.is_finished() && !parser.current_char().is_some_and(char::is_whitespace) {
            return Err(parser.error("expected whitespace after list element"));
        }
        parser.skip_whitespace();
    }
    Ok(words)
}

fn normalize_tcl_continuations(source: &str) -> String {
    let mut characters = source.chars().peekable();
    let mut normalized = String::with_capacity(source.len());
    while let Some(character) = characters.next() {
        if character == '\\' && matches!(characters.peek(), Some('\n' | '\r')) {
            if characters.next() == Some('\r') && matches!(characters.peek(), Some('\n')) {
                characters.next();
            }
            while characters
                .peek()
                .is_some_and(|character| character.is_whitespace())
            {
                characters.next();
            }
            normalized.push(' ');
        } else {
            normalized.push(character);
        }
    }
    normalized
}

struct ListParser<'a> {
    source: &'a str,
    offset: usize,
}

impl ListParser<'_> {
    fn word(&mut self) -> Result<String, StaticTclListError> {
        match self.current_char() {
            Some('{') => self.braced_word(),
            Some('"') => self.quoted_word(),
            Some(_) => self.bare_word(),
            None => Err(self.error("expected list element")),
        }
    }

    fn braced_word(&mut self) -> Result<String, StaticTclListError> {
        self.advance_char();
        let mut depth = 1_usize;
        let mut word = String::new();
        while let Some(character) = self.current_char() {
            self.advance_char();
            match character {
                '\\' => {
                    word.push(character);
                    let Some(escaped) = self.current_char() else {
                        return Err(self.error("unterminated escape in braced list element"));
                    };
                    self.advance_char();
                    word.push(escaped);
                }
                '{' => {
                    depth += 1;
                    word.push(character);
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(word);
                    }
                    word.push(character);
                }
                _ => word.push(character),
            }
        }
        Err(self.error("unterminated braced list element"))
    }

    fn quoted_word(&mut self) -> Result<String, StaticTclListError> {
        self.advance_char();
        let mut word = String::new();
        while let Some(character) = self.current_char() {
            self.advance_char();
            match character {
                '"' => return Ok(word),
                '\\' => word.push(self.escaped_character()?),
                _ => word.push(character),
            }
        }
        Err(self.error("unterminated quoted list element"))
    }

    fn bare_word(&mut self) -> Result<String, StaticTclListError> {
        let mut word = String::new();
        while let Some(character) = self.current_char() {
            if character.is_whitespace() {
                break;
            }
            self.advance_char();
            if character == '\\' {
                word.push(self.escaped_character()?);
            } else {
                word.push(character);
            }
        }
        Ok(word)
    }

    fn escaped_character(&mut self) -> Result<char, StaticTclListError> {
        let Some(character) = self.current_char() else {
            return Err(self.error("unterminated list escape"));
        };
        self.advance_char();
        Ok(character)
    }

    fn skip_whitespace(&mut self) {
        while self.current_char().is_some_and(char::is_whitespace) {
            self.advance_char();
        }
    }

    fn current_char(&self) -> Option<char> {
        self.source[self.offset..].chars().next()
    }

    fn advance_char(&mut self) {
        if let Some(character) = self.current_char() {
            self.offset += character.len_utf8();
        }
    }

    fn is_finished(&self) -> bool {
        self.offset == self.source.len()
    }

    fn error(&self, message: &'static str) -> StaticTclListError {
        StaticTclListError {
            offset: self.offset,
            message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_static_tcl_list_words_without_evaluation() {
        assert_eq!(
            parse_static_tcl_list(r#"-f step.cmd "quoted value" {sim run; puts done}"#).unwrap(),
            ["-f", "step.cmd", "quoted value", "sim run; puts done"]
        );
        assert_eq!(
            parse_static_tcl_list(r#"mkTop helper.c path\ with\ spaces"#).unwrap(),
            ["mkTop", "helper.c", "path with spaces"]
        );
        assert_eq!(
            parse_static_tcl_list("first.expected \\\n                second.expected").unwrap(),
            ["first.expected", "second.expected"]
        );
    }

    #[test]
    fn rejects_malformed_or_concatenated_list_elements() {
        assert!(parse_static_tcl_list("{unterminated").is_err());
        assert!(parse_static_tcl_list("{one}two").is_err());
    }
}
