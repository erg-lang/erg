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
    <script> 从脚本文件读取程序
            参数也可以指定要传递给 <script>

OPTIONS
    --help/-?/-h                         显示帮助
    --version/-V                         显示版本
    --verbose 0|1|2                      指定细致程度
    --opt-level/-o 0|1|2|3               指定优化级别
    --python-version/-p (uint 32 number) Python 版本
    --py-server-timeout (uint 64 number) Python REPL 服务器超时
    --dump-as-pyc                        转储为 .pyc 文件
    --mode lex|parse|compile|exec        执行模式

SUBCOMMAND
    -c cmd : 作为字符串传入程序
    -m mod : 要执行的模块",

    "traditional_chinese" =>
        "\
USAGE:
    erg [OPTIONS] [SUBCOMMAND] [ARGS]...

ARGS:
    <script> 從腳本檔案讀取程式
            參數也可以指定要傳遞給 <script>

OPTIONS
    --help/-?/-h                         顯示幫助
    --version/-V                         顯示版本
    --verbose 0|1|2                      指定細緻程度
    --opt-level/-o 0|1|2|3               指定優化級別
    --python-version/-p (uint 32 number) Python 版本
    --py-server-timeout (uint 64 number) Python REPL 服務器超時
    --dump-as-pyc                        轉儲為 .pyc 文件
    --mode lex|parse|compile|exec        執行模式

SUBCOMMAND
    -c cmd : 作為字串傳入程式
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
    脱糖しAST(抽象構文木)を返す

lower
    parseを実行し、ASTを獲得
    名前解決、型検査・型推論をしてHIR(高レベル中間表現)を返す

check
    lowerを実行
    副作用、所有権を確認しHIRを返す

compile
    checkを実行
    HIRをからバイトコードを生成し、<filename>.pycを出力する

exec
    compileを実行し、更に<filename>.pycを実行

read
    <filename>.pycをデシリアライズしコードオブジェクトの情報をダンプ",

    "simplified_chinese" =>
    "\
USAGE:
    erg --mode [lex | parse | lower | check | compile | exec | read] [SUBCOMMAND] [ARGS]...

lex
    从 <filename>.er, REPL 等接受输入, 并标记文本
    以TokenStream形式返回分析结果

parse
    执行 lex, 获取 TokenStream, 并解析语法
    将多模式定义语句的语法糖按匹配转换并返回AST(抽象语法树)

lower
    执行 parse
    解析名称、检查类型和推断, 并返回HIR(高级中间表示)

check
    执行 lower
    检查副作用、所有权并返回HIR

compile
    运行 check 以获取检查完成的 AST
    编译 AST 并返回 <文件名>.pyc

exec
    运行 check 以获取检查完成的 AST
    在执行 <filename>.pyc 后删除 <文件名>.pyc

read
    反序列化<文件名> .pyc 和 dump",

    "traditional_chinese" =>
    "\
USAGE:
        erg --mode [lex | parse | lower | check | compile | exec | read] [SUBCOMMAND] [ARGS]...

lex
    從 <檔名>.er, REPL 等接受輸入, 並標記文字
    以 TokenStream 形式返回分析結果

parse
    執行 lex, 獲取 TokenStream, 並解析語法
    將多模式定義語句的語法糖按匹配轉換並返回 AST(抽象語法樹)

lower
    執行 parse 以獲取AST
    解析名稱、檢查類型和推斷, 並返回HIR

check
    執行 lower 並獲取 AST
    檢查副作用、所有權並返回 AST

compile
    運行 check 以獲取檢查完成的 AST
    編譯 AST 並返回 <檔名>.pyc

exec
    運行check以獲取檢查完成的AST
    在執行<檔名>.pyc後删除<檔名>.pyc

read
    反序列化 <檔名>.pyc 和 dump",

    "english" =>
    "\
USAGE:
    erg --mode [lex | parse | lower | check | compile | exec | read] [SUBCOMMAND] [ARGS]...

lex
    Receive input from <filename>.er, REPL, etc., and tokenize the text
    Returns analysis results as TokenStream

parse
    Execute lexing, parsing & desugaring then return AST (abstract syntax tree)

lower
    Execute parsing
    Resolve name, check type and infer, and return HIR (High-Level Intermediate Representation)

check
    Execute lowering
    Check side-effects, ownership and return HIR

compile
    Execute check
    Compile HIR and return <filename>.pyc

exec
    Execute compiling & executing <filename>.pyc

read
    Deserialize <filename>.pyc and dump",
    )
}
