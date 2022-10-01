use crate::switch_lang;

/// erg -h/--help/-?
pub fn command_message<'a>() -> &'a str {
    switch_lang!(
        "japanese" =>
        "\
USAGE:
    erg [OPTIONS] [SUBCOMMAND] [ARGS]...

ARGS:
    <script> スクリプトファイルからプログラムを読み込む
             <script>に渡す引数を入力する

OPTIONS
    --help/-?/-h                         このhelpを表示
    --version/-V                         バージョンを表示
    --verbose 0|1|2                      冗長性レベルを指定
    --opt-level/-o 0|1|2|3               最適化レベルを指定
    --python-version/-p (uint 32 number) Pythonバージョンを指定
    --py-server-timeout (uint 64 number) PythonのREPLサーバーのタイムアウト時間を指定
    --dump-as-pyc                        .pycファイルにダンプ
    --mode lex|parse|compile|exec        指定モードで実行

SUBCOMMAND
    -c cmd : 文字列をプログラムに譲渡
    -m mod : モジュールを実行",

    "simplified_chinese" =>
        "\
USAGE:
    erg [OPTIONS] [SUBCOMMAND] [ARGS]...

ARGS:
    <script> 從腳本文件中讀取的程序
             參數也可以指定傳遞給 <script>

OPTIONS
    --help/-?/-h                         显示此帮助
    --version/-V                         显示版本
    --verbose 0|1|2                      详细程度
    --opt-level/-o 0|1|2|3               优化级别
    --python-version/-p (uint 32 number) Python版本
    --py-server-timeout (uint 64 number) Python REPL服务器超时
    --dump-as-pyc                        转储为 .pyc 文件
    --mode lex|parse|compile|exec        执行模式

SUBCOMMAND
    -c cmd : 作为字符串传入的程序
    -m mod : 要执行的模块",

    "traditional_chinese" =>
        "\
USAGE:
    erg [OPTIONS] [SUBCOMMAND] [ARGS]...

ARGS:
    <script> 从脚本文件中读取的程序
             参数也可以指定传递给 <script>

OPTIONS
    --help/-?/-h                         顯示此幫助
    --version/-V                         顯示版本
    --verbose 0|1|2                      詳細程度
    --opt-level/-o 0|1|2|3               優化級別
    --python-version/-p (uint 32 number) Python 版本
    --py-server-timeout (uint 64 number) Python REPL 服務器超時
    --dump-as-pyc                        轉儲為 .pyc 文件
    --mode lex|parse|compile|exec        執行模式

SUBCOMMAND
    -c cmd : 作為字符串傳入的程序
    -m mod : 要執行的模塊",

    "english" =>
        "\
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
    -m mod : module to be executed",
    )
}

pub fn mode_message<'a>() -> &'a str {
    switch_lang!(
        "japanese" =>
        "\
USAGE:
    erg --mode [lex | parse | lower | check | compile | exec | read] [SUBCOMMAND] [ARGS]...

lex
    <filename>.erやREPLなどから入力を受け取り、字句を解析
    解析結果をTokenStreamとして返す

parse
    lexを実行し、TokenStreamを獲得して構文を解析
    脱糖衣し複数パターン定義文をmatchで変換しast(抽象構文木)を返す

lower
    parseを実行し、astを獲得
    名前解決、型チェックと推論しastを返す

check
    lowerを実行しastを獲得
    副作用、所有権を確認しastを返す

compile
    checkを実行しチェックされたastを獲得
    astをコンパイルし、<filename>.pycを返す

exec
    checkを実行しチェックされたastを獲得
    <filename>.pycを実行後、<filename>.pycを削除

read
    <filename>.pycをデシリアライズしダンプ",

    "traditional_chinese" =>
        "\
USAGE:
    erg --mode [lex | parse | lower | check | compile | exec | read] [SUBCOMMAND] [ARGS]...

lex
    从 <filename>.er、REPL 等接收输入，并标记文本
    以 TokenStream 形式返回分析结果

parse
    执行lex，获取TokenStream，解析语法
    对多个模式定义语句进行脱糖，通过匹配转换并返回 ast（抽象语法树）

lower
    执行 parse 得到 ast
    解析名称，检查类型和推断，并返回 ast

check
    执行lower并get ast
    检查副作用、所有权和回报

compile
    運行檢查以檢查 ast
    編譯 ast 並返回 <filename>.pyc

exec
    执行检查以检查 ast
    执行 <filename>.pyc 后删除 <filename>.pyc

read
    反序列化 <filename>.pyc 并转储",

    "simplified_chinese" =>
        "\
USAGE:
    erg --mode [lex | parse | lower | check | compile | exec | read] [SUBCOMMAND] [ARGS]...

lex
    從 <filename>.er、REPL 等接收輸入，並標記文本
    以 TokenStream 形式返回分析結果

parse
    執行lex，獲取TokenStream，解析語法
    對多個模式定義語句進行脫糖，通過匹配轉換並返回 ast（抽象語法樹）

lower
    執行 parse 得到 ast
    解析名稱，檢查類型和推斷，並返回 ast

check
    執行lower並get ast
    檢查副作用、所有權和回報

compile
    运行检查以检查 ast
    编译 ast 并返回 <filename>.pyc

exec
    執行檢查以檢查 ast
    執行 <filename>.pyc 後刪除 <filename>.pyc

read
    反序列化 <filename>.pyc 並轉儲",

    "english" =>
        "\
USAGE:
    erg --mode [lex | parse | lower | check | compile | exec | read] [SUBCOMMAND] [ARGS]...

lex
    Receive input from <filename>.er, REPL, etc., and tokenize the text
    Returns analysis results as TokenStream

parse
    Execute lex, get TokenStream, and parse the syntax
    Desugar multiple pattern definition sentences, convert by match and return ast (abstract syntax tree)

lower
    Execute parse to get ast
    Resolve name, check type and infer, and return ast

check
    Execute lower and get ast
    Check side-effects, ownership and return ast

compile
    Run check to get checked ast
    Compile ast and return <filename>.pyc

exec
    Execute check to get checked ast
    Delete <filename>.pyc after executing <filename>.pyc

read
    Deserialize <filename>.pyc and dump",
    )
}
