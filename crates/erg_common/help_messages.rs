use crate::switch_lang;

/// erg -h/--help/-?
pub fn command_message<'a>() -> &'a str {
    switch_lang!(
        "japanese" =>
        "\
USAGE:
    erg [OPTIONS] [COMMAND] [ARGS]...

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
    --mode (mode)                        指定モードで実行(詳細は--mode --helpを参照)
    --code/-c (string)                   文字列として渡したプログラムを実行
    --module/-m (string)                 モジュールを実行

COMMAND
    lex                                  字句解析
    parse                                構文解析
    typecheck|tc                         型検査
    check                                全ての検査(所有権検査, 副作用検査などを含む)
    compile                              コンパイル
    transpile                            トランスパイル
    run|exec                             実行(デフォルト)
    server                               言語サーバーを起動
    lint                                 Lintを実行
    pack                                 パッケージング管理",

    "simplified_chinese" =>
    "\
USAGE:
    erg [OPTIONS] [COMMAND] [ARGS]...

ARGS:
    <script> 从脚本文件读取程序
            参数也可以指定要传递给 <script>

OPTIONS
    --help/-?/-h                         显示帮助
    --version/-V                         显示版本
    --verbose 0|1|2                      指定细致程度
    --opt-level/-o 0|1|2|3               指定优化级别
    --python-version/-p (uint 32 number) Python 版本
    --py-server-timeout (uint 64 number) 指定等待 REPL 输出的秒数
    --dump-as-pyc                        转储为 .pyc 文件
    --mode (mode)                        执行模式 (更多信息见`--mode --help`)
    --code/-c (string)                   作为字符串传入程序
    --module/-m (string)                 要执行的模块

COMMAND
    lex                                  字词解析
    parse                                语法解析
    typecheck|tc                         类型检查
    check                                全部检查(包括所有权检查, 副作用检查等)
    compile                              编译
    transpile                            转译
    run|exec                             执行(默认模式)
    server                               执行语言服务器
    lint                                 执行 Lint
    pack                                 执行打包管理",

    "traditional_chinese" =>
        "\
USAGE:
    erg [OPTIONS] [COMMAND] [ARGS]...

ARGS:
    <script> 從腳本檔案讀取程式
            參數也可以指定要傳遞給 <script>

OPTIONS
    --help/-?/-h                         顯示幫助
    --version/-V                         顯示版本
    --verbose 0|1|2                      指定細緻程度
    --opt-level/-o 0|1|2|3               指定優化級別
    --python-version/-p (uint 32 number) Python 版本
    --py-server-timeout (uint 64 number) 指定等待 REPL 輸出的秒數
    --dump-as-pyc                        轉儲為 .pyc 文件
    --mode (mode)                        執行模式 (更多信息見`--mode --help`)
    --code/-c (string)                   作為字串傳入程式
    --module/-m (string)                 要執行的模塊

COMMAND
    lex                                  字詞解析
    parse                                語法解析
    typecheck|tc                         型檢查
    check                                全部檢查(包括所有權檢查, 副作用檢查等)
    compile                              編譯
    transpile                            轉譯
    run|exec                             執行(預設模式)
    server                               執行語言伺服器
    lint                                 執行 Lint
    pack                                 執行打包管理",

    "english" =>
        "\
USAGE:
    erg [OPTIONS] [COMMAND] [ARGS]...

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
    --mode (mode)                        execution mode (See `--mode --help` for details)
    --code/-c (string)                   program passed in as string
    --module/-m (string)                 module to be executed

COMMAND
    lex                                  lexical analysis
    parse                                syntax analysis
    typecheck|tc                         type check
    check                                full check (including ownership check, effect check, etc.)
    compile                              compile
    transpile                            transpile
    run|exec                             execute (default mode)
    server                               start Erg language server
    lint                                 lint
    pack                                 run package manager",
    )
}

pub fn mode_message<'a>() -> &'a str {
    switch_lang!(
        "japanese" =>
        "\
USAGE:
    erg --mode [lex | parse | lower | check | compile | exec | read | lint | pack] [SUBCOMMAND] [ARGS]...

lex
    <filename>.erやREPLなどから入力を受け取り、字句を解析
    解析結果をTokenStreamとして返す

parse
    lexを実行し、TokenStreamを獲得して構文を解析
    脱糖しAST(抽象構文木)を返す

typecheck/lower
    parseを実行し、ASTを獲得
    名前解決、型検査・型推論をしてHIR(高レベル中間表現)を返す

check
    lowerを実行
    副作用、所有権を確認しHIRを返す

compile
    checkを実行
    HIRからバイトコードを生成し、<filename>.pycを出力する

transpile
    checkを実行
    HIRからPythonスクリプトを生成し、<filename>.pyを出力

run/exec
    compileを実行し、更に<filename>.pycを実行

read
    <filename>.pycをデシリアライズしコードオブジェクトの情報をダンプ

lint
    プログラムをLintする

pack
    パッケージ管理",

    "simplified_chinese" =>
    "\
USAGE:
    erg --mode [lex | parse | lower | check | compile | exec | read] [SUBCOMMAND] [ARGS]...

lex
    从 <filename>.er, REPL 等接受输入, 并标记文本
    以 TokenStream 形式返回分析结果

parse
    执行 lex, 获取 TokenStream, 并解析语法
    将多模式定义语句的语法糖按匹配转换并返回 AST(抽象语法树)

typecheck/lower
    执行 parse
    解析名称、检查类型和推断, 并返回 HIR(高级中间表示)

check
    执行 lower
    检查副作用、所有权并返回 HIR

compile
    运行 check 以获取检查完成的 AST
    编译 AST 并返回 <文件名>.pyc

transpile
    运行 check 以获取检查完成的 AST
    将 AST 转换为 Python 代码并返回 <文件名>.py

run/exec
    运行 check 以获取检查完成的 AST
    在执行 <文件名>.pyc 后删除 <文件名>.pyc

read
    反序列化 <文件名>.pyc 和 dump

lint
    Lint 程序

pack
    包管理",

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

typecheck/lower
    執行 parse
    解析名稱、檢查類型和推斷, 並返回 HIR(高級中間表示)

check
    執行 lower
    檢查副作用、所有權並返回 HIR

compile
    運行 check 以獲取檢查完成的 AST
    編譯 AST 並返回 <檔名>.pyc

transpile
    運行 check 以獲取檢查完成的 AST
    從 HIR 生成 Python 腳本並返回 <檔名>.py

exec
    運行check以獲取檢查完成的 AST
    在執行 <檔名>.pyc 後删除 <檔名>.pyc

read
    反序列化 <檔名>.pyc 和 dump

lint
    Lint 程式

pack
    封裝管理",

    "english" =>
    "\
USAGE:
    erg --mode [lex | parse | lower | check | compile | exec | read] [SUBCOMMAND] [ARGS]...

lex
    Receive input from <filename>.er, REPL, etc. and lex the text
    Returns the analysis results as a TokenStream

parse
    Executes lex to get TokenStream, and parses it
    Degenerate and return AST (Abstract Syntax Tree)

typecheck/lower
    Execute parse to obtain AST
    Performs name resolution, type checking, and type inference, and returns HIR (High-level Intermediate Representation)

check
    Execute lower
    Checks for side-effects, ownership, and returns HIR

compile
    Execute check
    Generates bytecode from HIR and outputs <filename>.pyc

transpile
    Execute check
    Generates Python script from HIR and outputs <filename>.py

run/exec
    Execute compile and then <filename>.pyc

read
    Deserialize <filename>.pyc and dump code object information

lint
    Lint the program

pack
    Package management",
    )
}

pub const OPTIONS: &[&str] = &[
    "--build-features",
    "-c",
    "--code",
    "--check",
    "--compile",
    "--dest",
    "--dump-as-pyc",
    "--language-server",
    "--no-std",
    "--help",
    "-?",
    "-h",
    "--hex-py-magic-num",
    "--hex-python-magic-number",
    "--mode",
    "--module",
    "-m",
    "--optimization-level",
    "--opt-level",
    "-o",
    "--output-dir",
    "--ping",
    "--ps1",
    "--ps2",
    "--python-version",
    "-p",
    "--py-server-timeout",
    "--py-command",
    "--python-command",
    "--py-magic-num",
    "--python-magic-number",
    "--quiet-startup",
    "--quiet-repl",
    "--show-type",
    "-t",
    "--target-version",
    "--version",
    "-V",
    "--verbose",
];
