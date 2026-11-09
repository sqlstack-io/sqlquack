//! Name spellings for construct keys.

pub(crate) fn snake_case(camel: &str) -> String {
    let mut snake = String::with_capacity(camel.len() + 4);
    for (index, character) in camel.chars().enumerate() {
        if character.is_ascii_uppercase() {
            if index > 0 {
                snake.push('_');
            }
            snake.push(character.to_ascii_lowercase());
        } else {
            snake.push(character);
        }
    }
    snake
}
