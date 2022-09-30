/// erg -h/--help/-?
#[cfg(not(feature = "japanese"))]
pub const CMD_HELP: &str = "\
USAGE:
    erg [OPTIONS] [SUBCOMMAND] [ARGS]...

ARGS:
    <script> program read from script file
             Arguments can also be specified to be passed to the <script>

OPTIONS
    --help/-?/-h                         show this help
    --version/-V                         show version
    --verbose 0|1|2                      verbosity level
    --opt-level/-o 0|1|2|3               optimization level
    --python-version/-p (uint 32 number) Python version
    --py-server-timeout (uint 64 number) timeout for the Python REPL server
    --dump-as-pyc                        dump as .pyc file
    --mode lex|parse|compile|exec        execution mode

SUBCOMMAND
    -c cmd : program passed in as string
    -m mod : module to be executed";

#[cfg(feature = "japanese")]
pub const CMD_HELP: &str = "\
USAGE:
    erg [OPTIONS] [SUBCOMMAND] [ARGS]...

ARGS:
    <script> スクリプトファイルからプログラムを読み込む
             <script>に渡す引数を入力する

OPTIONS
    --help/-?/-h                         このhelpを表示
    --version/-V                         バージョンを表示
    --verbose 0|1|2                      冗長化レベル
    --opt-level/-o 0|1|2|3               最適化レベル
    --python-version/-p (uint 32 number) Pythonバージョンを指定
    --py-server-timeout (uint 64 number) PythonのREPLサーバーのタイムアプト時間を指定
    --dump-as-pyc                        .pycファイルにダンプ
    --mode lex|parse|compile|exec        モードで実行

SUBCOMMAND
    -c cmd : 文字列をプログラムに渡す
    -m mod : モジュールを実行させる";

#[cfg(feature = "simplified_chinese")]
pub const CMD_HELP: &str = "\
USAGE:
    erg [OPTIONS] [SUBCOMMAND] [ARGS]...

ARGS:
    <script> program read from script file
             Arguments can also be specified to be passed to the <script>

OPTIONS
    --help/-?/-h                         show this help
    --version/-V                         show version
    --verbose 0|1|2                      verbosity level
    --opt-level/-o 0|1|2|3               optimization level
    --python-version/-p (uint 32 number) Python version
    --py-server-timeout (uint 64 number) timeout for the Python REPL server
    --dump-as-pyc                        dump as .pyc file
    --mode lex|parse|compile|exec        execution mode

SUBCOMMAND
    -c cmd : program passed in as string
    -m mod : module to be executed";

#[cfg(feature = "traditional_chinese")]
pub const CMD_HELP: &str = "\
USAGE:
    erg [OPTIONS] [SUBCOMMAND] [ARGS]...

ARGS:
    <script> program read from script file
             Arguments can also be specified to be passed to the <script>

OPTIONS
    --help/-?/-h                         show this help
    --version/-V                         show version
    --verbose 0|1|2                      verbosity level
    --opt-level/-o 0|1|2|3               optimization level
    --python-version/-p (uint 32 number) Python version
    --py-server-timeout (uint 64 number) timeout for the Python REPL server
    --dump-as-pyc                        dump as .pyc file
    --mode lex|parse|compile|exec        execution mode

SUBCOMMAND
    -c cmd : program passed in as string
    -m mod : module to be executed";

/// erg --mode -h/--help/-?
#[cfg(not(feature = "japanese"))]
pub const MODE_HELP: &str = "\
USAGE:
    erg --mode [lex | parse | lower | check | compile | exec | read] [SUBCOMMAND] [ARGS]...

lex
    Takes input from a .er file or REPL strings, and lexes them into erg tokens
    The lexed string becomes an iterator as TokenStream

parse
    Exec lex and get TokenStream
    Iterate TokenStream and confirm that it fits the erg grammar
    TokenStream is reduced to Module

lower
    Exec parse and get module
    Module is converted ast(abstract syntax tree)

check
    Exec lower and get module
    Borrow checks at ast

compile
    Exec check and get ast
    Compile ast to o.pyc file

exec
    exec the compiled o.pyc

read
    read the .pyc, and out put to .er file";

#[cfg(feature = "japanese")]
pub const MODE_HELP: &str = "\
USAGE:
    erg --mode [lex | parse | lower | check | compile | exec | read] [SUBCOMMAND] [ARGS]...

lex
    .erファイルやREPLなどから入力を受け取り、それを字句解析
    字句解析された文字列のtokenをTokenStreamとして収集

parse
    lexを実行し、TokenStreamを獲得
    TokenStreamをイテレートし、ergの構文解析
    TokenStreamをmoduleに還元

lower
    parseを実行し、moduleを獲得
    moduleをast(抽象構文木)に変換

check
    lowerを実行しastを獲得
    astの借用チェック

compile
    checkを実行し借用チェックされたastを獲得
    astをo.pycにコンパイル

exec
    compileを実行し、o.pycを獲得
    o.pycを実行

read
    o.pycを読み取り、ergのastに変換";

#[cfg(feature = "simplified_chinese")]
pub const MODE_HELP: &str = "\
USAGE:
    erg --mode [lex | parse | lower | check | compile | exec | read] [SUBCOMMAND] [ARGS]...

lex
    TODO
    TODO

parse
    TODO
    TODO
    TODO

lower
    TODO
    TODO

check
    TODO
    TODO

compile
    TODO
    TODO

exec
    TODO
    TODO

read
    TODO";

#[cfg(feature = "traditional_chinese")]
pub const MODE_HELP: &str = "\
USAGE:
    erg --mode [lex | parse | lower | check | compile | exec | read] [SUBCOMMAND] [ARGS]...

lex
    TODO
    TODO

parse
    TODO
    TODO
    TODO

lower
    TODO
    TODO

check
    TODO
    TODO

compile
    TODO
    TODO

exec
    TODO
    TODO

read
    TODO";
