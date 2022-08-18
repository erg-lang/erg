# Multilingualization of Messages

Erg is working on making all messages (start, option, doc, hint, warning, error messages, etc.) multilingual within the language.
This project is open to anyone without detailed knowledge of Rust or Erg. Your participation is always welcome.

Here is how to translate them.

## Search `switch_lang!`

In the Erg source code, look for the item `switch_lang!` (use grep or your editor's search function).
You should find something like this:

```rust
switch_lang!(
    "japanese" => format!("この機能({name})はまだ正式に提供されていません"),
    "english" => format!("this feature({name}) is not implemented yet"),
),
```

This message is currently supported only in Japanese and English. Let's add a simplified Chinese message as a test.

## Add a New Message

Add translated messages as you see the content in other languages. Don't forget the comma (`,`) last.

```rust
switch_lang!(
    "japanese" => format!("この機能({name})はまだ正式に提供されていません"),
    "simplified_chinese" => format!("该功能({name})还没有正式提供"),
    "english" => format!("this feature({name}) is not implemented yet"),
),
```

Note that English is the default and must come last.
The `{name}` part is a Rust formatting feature that allows you to embed the contents of a variable (`name`) into a string.

## Build

Now, let's build with the `--features simplified_chinese` option.

<img src="../../../assets/screenshot_i18n_messages.png" alt='screenshot_i18n_messages'>

We did it!

## FAQ

Q: What does a specification like `{RED}{foo}{RESET}` mean?
A: {RED} and subsequent letters will be displayed in red. {RESET} will restore the color.

Q: If I want to add my language, how do I replace the `"simplified_chinese" =>` part?

The following languages are currently supported:

* "english" (default)
* "japanese"
* "simplified_chinese"
* "traditional_chinese"

If you would like to add languages other than these, please make a request.
