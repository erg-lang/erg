# Multilingualization of Messages

Erg is making its messages (start, options, documentation, hints, warnings, error messages, etc.) multilingual.
You don't need detailed knowledge of Rust or Erg to participate in this project. We appreciate your cooperation.

The method for multilingualization is explained below.

## Look for `switch_lang!`

Find the entry `switch_lang!` in the Erg source code (use grep or your editor's search function).
You should find something like this:

```rust
switch_lang!(
    "japanese" => format!("This feature ({name}) is not officially available yet"),
    "english" => format!("this feature({name}) is not implemented yet"),
),
```

This message is currently supported in Japanese and English only. Let's try adding a simplified Chinese message.

## add a message

Add translated messages while viewing content in other languages. Don't forget the comma (`,`) at the end.

```rust
switch_lang!(
    "japanese" => format!("This feature ({name}) is not officially available yet"),
    "simplified_chinese" => format!("This function ({name}) has been officially provided"),
    "english" => format!("this feature({name}) is not implemented yet"),
),
```

Note that English is the default and should always come last.
The `{name}` part is Rust's formatting feature that allows you to embed the contents of a variable (`name`) into a string.

## Build

Now let's build with the `--features simplified_chinese` option.

<img src="https://raw.githubusercontent.com/erg-lang/erg/main/assets/screenshot_i18n_messages.png" alt='screenshot_i18n_messages'>

You did it!

## FAQs

Q: What does a specification like `{RED}{foo}{RESET}` mean?
A: Everything after {RED} is displayed in red. {RESET} restores the color.

Q: If I want to add my own language, how do I replace the `"simplified_chinese" =>` part?
A: We currently support the following languages:

* "english" (default)
* "japanese" (Japanese)
* "simplified_chinese" (Simplified Chinese)
* "traditional_chinese" (Traditional Chinese)

If you would like to add languages ​​other than these, please make a request.
