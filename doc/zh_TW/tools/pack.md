# 包管理器

Erg 標配包管理器，可通過子命令調用。以下是典型的選項。

* ：將當前目錄初始化為軟件包。生成文件和<gtr=“6”/>目錄。指定<gtr=“7”/>是可執行文件的軟件包，指定<gtr=“8”/>是庫的軟件包，指定<gtr=“9”/>是兩個軟件包。如果指定<gtr=“10”/>，它將自動放置許可證文件。
* ：構建包。執行測試並進行優化。工件配置在或<gtr=“14”/>。
* ：安裝軟件包。對於庫，被放置在<gtr=“17”/>以下，應用程序被放置在<gtr=“18”/>作為 shell 腳本。添加時進行最優化。
* ：構建軟件包並運行應用程序（僅限 app 軟件包）。
* 刪除：build 目錄中的內容。
* ：測試軟件包。有關詳細信息，請參見。
* ：發布/發佈軟件包。我需要 GitHub 帳戶和公鑰。

另外，本文檔說明了管理自己的軟件包時的方法。要安裝或搜索外部軟件包，請參閱。此外，有關 Erg 的封裝系統，請參見<gtr=“26”/>。

## 整個軟件包的標準目錄配置（適用於應用程序軟件包）


```console
/package # パッケージのルートディレクトリ
    /build # ビルド結果を格納するディレクトリ
        /debug # デバッグビルド時の成果物
        /release # リリースビルド時の成果物
    /doc # ドキュメント(さらに`en`, `ja`などのサブディレクトリに分けることで各國語対応可能)
    /src # ソースコード
        /main.er # main関數を定義するファイル
    /tests # (ブラックボックス)テストファイルを格納するディレクトリ
    /package.er # パッケージの設定を定義するファイル
```

## package.er

如果，就會生成以下文件<gtr=“28”/>。 <gtr=“29”/>中記述了軟件包的設定。以下是<gtr=“30”/>的記述例子。


```erg
name = "example" # package name
author = "John Smith" # package author name
version = "0.1.0"
description = "An awesome package"
categories = ["cli"] # package categories
type = "app" # "app" or "lib"
license = "" # e.g. "MIT", "APACHE-2.0", "MIT OR Apache-2.0"
pre_build = "" # script filename to be executed before build
post_build = "" # script filename to be executed after build
dependencies = {
    # The latest one is selected if the version is not specified
    # If the version specification is omitted, the package manager automatically adds the version of the last successful build to the comments
    foo  = pack("foo") # [INFO] the last successfully built version: 1.2.1
    # Packages can be renamed
    bar1 = pack("bar", "1.*.*") # [INFO] the last successfully built version: 1.2.0
    bar2 = pack("bar", "2.*.*") # [INFO] the last successfully built version: 2.0.0
    baz  = pack("baz", "1.1.0")
}
deprecated = False
successors = [] # alternative packages (when a package is deprecated)
```

## 語義

Erg 軟件包根據指定版本。語義版本化通常以<gtr=“31”/>格式指定，其中 x，y 和 z 是大於或等於 0 的整數。每個數字的含義如下。

* x：主要版本（增加一個，以進行破壞兼容性的更新）
* y：次要版本（兼容更新（如 API 添加或過時）時增加 1；補丁版本升級（如錯誤修復）
* z：修補程序版本（錯誤修復和兼容性較小的更改增加 1；嚴重的不兼容修復將在主要版本升級中提供）

但是，在默認情況下，對版本的更改始終不兼容。如果你想在版本升級時保持兼容性，請在後面指定<gtr=“34”/>（Erg 自己的規則）。例如，如果你想添加<gtr=“35”/>，同時兼容，即升級到<gtr=“36”/>，請指定<gtr=“37”/>。如果已修復錯誤，請指定<gtr=“38”/>。這將確保該版本與上一版本兼容。如果你想將<gtr=“39”/>升級為<gtr=“40”/>，也可以使用此選項。也就是說，<gtr=“41”/>與上一個版本<gtr=“42”/>兼容。

語義版本化在生成鎖定文件時非常重要。鎖定文件是為保持相關軟件包的兼容性而生成的文件，它依賴於舊軟件包，除非明確更新相關軟件包的新版本。當多人開發具有相關軟件包的軟件包時，鎖定文件非常有用。它還可以節省本地存儲，因為相關軟件包可以在兼容的情況下使用更多相關軟件包。

Erg 軟件包管理器嚴格執行以上規則，任何與規則相衝突的軟件包更新都將被拒絕。 Erg 包管理器與版本控制系統（如 git）配合使用，在 publish 包時檢測代碼差異，以驗證版本化的有效性。具體地說，包管理器看 API 的類型。如果類型是舊版本的子類型，則將更改視為兼容（請注意，這不是完全驗證。可能存在類型上兼容但語義上不兼容的更改。這是開發人員的工作。

此外，由於軟件包在註冊表中註冊了整個存儲庫，因此開發人員不能在不通過軟件包管理器的情況下更新軟件包。此外，軟件包可以過時，但不能刪除。

### Appendix：語義版本化問題及其對策

語義版本化中存在（至少）兩個已知問題。首先，語義版本化可能會施加過大的限制。在語義版本化中，只有一個不兼容的 API 更改會提升整個包的主要版本。這會導致“我想嘗試一個新的 API，但因為我必須處理另一個不兼容的 API 更改而推遲升級”。還有一點，語義版本可能承諾過高。如上一節所述，API 的“兼容更改”在理論上無法證明。如果你指定要版本的軟件包，則從語義版本的角度來看，可以使用大於或等於<gtr=“44”/>小於<gtr=“45”/>的所有軟件包（不能使用<gtr=“46”/>）。但是，實際上，軟件包開發人員無意中使用 API 可能會導致構建不成功。

為了解決這個問題，Erg 採取了一種方法，允許你同時使用不同版本的軟件包（通過重命名）。這允許你在引入部分版本 2 的 API 的同時繼續使用版本 1 的 API。此外，雖然不是很理想，但如果只有某些次要版本的 API 可以在沒有錯誤的情況下使用，則可以將其保留到下一個版本。

## publish

可以使用子命令發佈軟件包。我需要一個 GitHub 賬戶來發表。缺省情況下，軟件包註冊為<gtr=“48”/>。如果滿足一定的條件（下載數量，維護頻率等），則可以申請註冊省略所有者名稱的別名。軟件包名稱不區分大小寫和分隔符，如<gtr=“49”/>和<gtr=“50”/>。