//! Why an input does not tokenize.

/// Why the lossless tokenizer could not cover the input: the lexer's fault, as the engine reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanError {
    /// The engine's message.
    pub message: String,
    /// The engine's error number, such as 105 for an unclosed quotation mark.
    pub code: u32,
    /// Byte offset where the faulty token starts.
    pub position: usize,
}
