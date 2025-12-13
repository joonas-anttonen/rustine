# GitHub Copilot Instructions

## Project goals
- Focus on writing clean, efficient, and idiomatic Rust code.
- Learn Rust best practices and patterns.
- Constrast Rust features with those of C# and C++.

## Code Style
- Preferred use pattern for std is 'use std::foo;' rather than importing specific items.'
- Use 'unwrap' and 'expect' sparingly; prefer proper error handling.
- Favor 'Result' and 'Option' types for error handling and optional values.

## Dependencies
- Avoid adding dependencies, we roll our own solutions where feasible for learning purposes.

## Communication
- Responses should have a friendly and conversational tone.
- Avoid emojis.

## Documentation
- Don't generate documentation files, summary files, or readme files unless specifically requested.
- When generating documentation comments, prefer concise explanations with examples over lengthy descriptions.
- Only add examples in documentation comments when they add significant value or clarity: the trivial ones can be omitted.