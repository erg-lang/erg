# コマンドライン引数

## サブコマンド

### lex

字句解析結果を表示します。

### parse

構文解析結果を表示します。

### typecheck

型検査結果を表示します。

### compile

コンパイルを実行します。

### transpile

Pythonスクリプトへ変換します。

### run (exec)

実行結果を表示します。

### server

ランゲージサーバーを起動します。

## オプション

### --build-features

コンパイラのビルド時に有効化されたfeaturesを表示します。

### -c, --code

実行するコードを指定します。

### --dump-as-pyc

コンパイル結果を`.pyc`ファイルとして出力します。

### -?, -h, --help

ヘルプを表示します。

### --mode

サブコマンドの実行モードを指定します。

### -m, --module

実行するモジュールを指定します。

### --no-std

Erg標準ライブラリを使用しないでコンパイルします。

### -o, --opt-level

最適化レベルを指定します。0から3までの値を指定できます。

### --output-dir, --dest

コンパイル成果物の出力先ディレクトリを指定します。

### -p, --python-version

Pythonのバージョンを指定します。バージョン番号は32bit符号なし整数で、[このリスト](https://github.com/google/pytype/blob/main/pytype/pyc/magic.py)の中から選択してください。

### --py-command, --python-command

使用するPythonインタープリを指定します。デフォルトはUnixの場合`python3`、Windowsの場合`python`です。

### --py-server-timeout

REPL実行のタイムアウト時間を指定します。デフォルトは10秒です。

### --quiet-startup, --quiet-repl

REPL起動時に処理系の情報を表示しなくなります。

### -t, --show-type

REPLで実行結果とともに型情報を表示します。

### --target-version

出力するpycファイルのバージョンを指定します。バージョンはセマンティックバージョニングに従います。

### -V, --version

バージョンを表示します。

### --verbose

情報の冗長度を制御します。0から2までの値を指定できます。
0に指定しても警告は表示されます。

### --

実行時の引数を指定します。
