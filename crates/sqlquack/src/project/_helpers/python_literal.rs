//! A Python call's keyword arguments as JSON: the strings, numbers, booleans, None, lists and dicts a `@udf(...)` holds, each dict as its `[key, value]` pairs in written order.

use serde_json::{Map, Number, Value};

/// The call arguments `(keyword=literal, ...)` at the start of `text`, as an object.
pub(crate) fn python_keywords(text: &str) -> Result<Map<String, Value>, String> {
    Reader {
        characters: text.chars().collect(),
        index: 0,
    }
    .keywords()
}

struct Reader {
    characters: Vec<char>,
    index: usize,
}

impl Reader {
    /// The next character after spaces, line breaks and comments, without taking it.
    fn peek(&mut self) -> Option<char> {
        loop {
            match self.characters.get(self.index) {
                Some(character) if character.is_whitespace() => self.index += 1,
                Some('#') => {
                    while !matches!(self.characters.get(self.index), None | Some('\n')) {
                        self.index += 1;
                    }
                }
                other => return other.copied(),
            }
        }
    }

    fn take(&mut self, expected: char) -> Result<(), String> {
        match self.peek() {
            Some(found) if found == expected => {
                self.index += 1;
                Ok(())
            }
            Some(found) => Err(format!("expected `{expected}`, found `{found}`")),
            None => Err(format!("expected `{expected}`, found the end")),
        }
    }

    /// `(name=value, ...)`.
    fn keywords(&mut self) -> Result<Map<String, Value>, String> {
        self.take('(')?;
        let mut keywords: Map<String, Value> = Map::new();
        while self.peek() != Some(')') {
            let name = self.word();
            if name.is_empty() {
                return Err("expected a keyword argument".to_owned());
            }
            self.take('=')?;
            let value = self.value()?;
            keywords.insert(name, value);
            if self.peek() != Some(')') {
                self.take(',')?;
            }
        }
        self.index += 1;
        Ok(keywords)
    }

    fn value(&mut self) -> Result<Value, String> {
        match self.peek() {
            Some(quote @ ('\'' | '"')) => self.string(quote).map(Value::String),
            Some(open @ ('[' | '(')) => {
                self.index += 1;
                let close = if open == '[' { ']' } else { ')' };
                let mut items: Vec<Value> = Vec::new();
                while self.peek() != Some(close) {
                    items.push(self.value()?);
                    if self.peek() != Some(close) {
                        self.take(',')?;
                    }
                }
                self.index += 1;
                Ok(Value::Array(items))
            }
            Some('{') => {
                self.index += 1;
                let mut entries: Vec<Value> = Vec::new();
                while self.peek() != Some('}') {
                    let key = match self.value()? {
                        Value::String(key) => key,
                        other => return Err(format!("a dict key is not text: {other}")),
                    };
                    self.take(':')?;
                    entries.push(Value::Array(vec![Value::String(key), self.value()?]));
                    if self.peek() != Some('}') {
                        self.take(',')?;
                    }
                }
                self.index += 1;
                Ok(Value::Array(entries))
            }
            Some(_) => match self.word().as_str() {
                "True" => Ok(Value::Bool(true)),
                "False" => Ok(Value::Bool(false)),
                "None" => Ok(Value::Null),
                word => number(word)
                    .map(Value::Number)
                    .ok_or_else(|| format!("`{word}` is not a literal")),
            },
            None => Err("expected a value, found the end".to_owned()),
        }
    }

    /// A name or number: letters, digits, `_`, `.`, `+` and `-`.
    fn word(&mut self) -> String {
        self.peek();
        let start = self.index;
        while self.characters.get(self.index).is_some_and(|character| {
            character.is_alphanumeric() || matches!(character, '_' | '.' | '+' | '-')
        }) {
            self.index += 1;
        }
        self.characters[start..self.index].iter().collect()
    }

    /// A quoted string, single or triple, with backslash escapes; adjacent strings join.
    fn string(&mut self, quote: char) -> Result<String, String> {
        let triple = self.characters[self.index..].starts_with(&[quote, quote, quote]);
        let width = if triple { 3 } else { 1 };
        self.index += width;
        let mut text = String::new();
        loop {
            match self.characters.get(self.index).copied() {
                None => return Err("a string is not closed".to_owned()),
                Some('\\') => {
                    let escaped = self.characters.get(self.index + 1).copied();
                    text.push(match escaped {
                        Some('n') => '\n',
                        Some('t') => '\t',
                        Some(other) => other,
                        None => return Err("a string is not closed".to_owned()),
                    });
                    self.index += 2;
                }
                Some(character)
                    if character == quote
                        && (!triple
                            || self.characters[self.index..]
                                .starts_with(&[quote, quote, quote])) =>
                {
                    self.index += width;
                    break;
                }
                Some(character) => {
                    text.push(character);
                    self.index += 1;
                }
            }
        }
        match self.peek() {
            Some(next @ ('\'' | '"')) => Ok(text + &self.string(next)?),
            _ => Ok(text),
        }
    }
}

/// `text` as an integer or a finite float, or None when it is neither.
fn number(text: &str) -> Option<Number> {
    match (text.parse::<i64>(), text.parse::<f64>()) {
        (Ok(integer), _) => Some(Number::from(integer)),
        (Err(_not_integer), Ok(float)) => Number::from_f64(float),
        (Err(_not_integer), Err(_not_float)) => None,
    }
}
