# 包管理器

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/tools/pack.md%26commit_hash%3D06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/tools/pack.md&commit_hash=06f8edc9e2c0cee34f6396fd7c64ec834ffb5352)

Erg 標配有一個包管理器，您可以使用 `pack` 子命令調用它
以下是典型的選項

* `erg pack init`: 將當前目錄初始化為一個包。會生成一個 `package.er` 文件和一個 `src` 目錄。指定 `app` 將產生一個可執行包，`lib` 將產生一個庫包，而 `hybrid` 將產生兩個包。如果指定了 `--license`，將自動放置許可文件
* `erg pack build`: 構建一個包。使用 `--release` 可以運行和優化測試。工件放置在 `build/debug` 或 `build/release` 中
* `erg pack install`: 安裝一個包。在庫的情況下，`src` 被放置在 `.erg/lib` 中，而應用程序作為 shell 腳本被放置在 `.erg/app` 中。使用 `--release` 進行優化
* `erg pack run`: 構建包并運行應用程序(僅限應用程序包)
* `erg pack clean`: 刪除構建目錄的內容
* `erg pack test`: 運行包測試。有關詳細信息，請參閱 [test.md](./test.md)
* `erg pack publish`: 發布/發布包。您將需要一個 GitHub 帳戶和公鑰

本文檔解釋了如何管理您自己的包
如果要安裝或搜索外部包，請參閱 [install.md](./install.md)
另請參閱 [package_system.md](../syntax/33_package_system.md) 了解 Erg 包系統

## 整個包的標準目錄結構(對于應用程序包)

```console
/package # package root directory
    /build # Directory to store build results
        /debug # Artifacts during debug build
        /release # Artifacts of release build
    /doc # Documents (in addition, by dividing into subdirectories such as `en`, `ja` etc., it is possible to correspond to each language)
    /src # source code
        /main.er # file that defines the main function
    /tests # Directory to store (black box) test files
    /package.er # file that defines package settings
```

## package.er

`erg pack init` 將生成如下所示的 `package.er` 文件。 `package.er` 描述了包的配置
下面是一個`package.er`的例子

```python
name = "example" # package 名稱
author = "John Smith" # package 作者名稱
version="0.1.0"
description = "An awesome package"
categories = ["cli"] # package 類別
type = "app" # "app" 或者 "lib"
license = "" # 例如"MIT", "APACHE-2.0", "MIT OR Apache-2.0"
pre_build = "" # 構建前要執行的腳本文件名
post_build = "" # 構建后要執行的腳本文件名
dependencies = {
    # 如果不指定版本，則選擇最新的
    # 如果省略版本說明，包管理器會自動將上次成功構建的版本添加到注釋中
    foo = pack("foo") # [INFO] 最后成功構建的版本: 1.2.1
    # 包可以重命名
    bar1 = pack("bar", "1.*.*") # [INFO] 最后成功構建的版本: 1.2.0
    bar2 = pack("bar", "2.*.*") # [INFO] 最后成功構建的版本: 2.0.0
    baz = pack("baz", "1.1.0")
}
deprecated=False
successors = [] # 替代包(當一個包被棄用時)
```

## 語義版本控制

Erg 包是基于 [語義版本控制](https://semver.org/lang/en/) 進行版本控制的
語義版本控制大致以"x.y.z"格式指定(x、y、z 是大于或等于 0 的整數)
每個數字的含義如下

* x: 主要版本(更新破壞兼容性時增加 1)
* y: 次要版本(執行兼容更新時增加1(API添加，棄用等)，錯誤修復等由補丁版本升級處理)
* z: 補丁版本(當進行小的更改以修復錯誤或保持兼容性時增加1，破壞兼容性的嚴重修復由主要版本升級處理)

但是，默認情況下，版本 `0.*.*` 中的更改始終是不兼容的。如果要在保持兼容性的同時升級，請在其后指定 `-compatible`(Erg 自己的規則)。例如，如果要在保持與 0.2.1 兼容的同時添加功能，即要升級到 0.3.0，則指定 0.3.0-compatible。如果您已修復錯誤，還請指定"0.2.2-compatible"
該版本將被視為與以前的版本兼容
即使您想將 `0.*.*` 升級到 `1.0.0`，這仍然有效。也就是說，`1.0.0-compatible` 與之前的版本 `0.y.z` 兼容

生成鎖文件時，語義版本控制非常重要。鎖定文件是為保持依賴項兼容而生成的文件，因此除非明確更新，否則較新版本的依賴項依賴于較舊的包
當多人開發具有依賴包的包時，鎖定文件很有用。它還通過允許依賴于它們的包在兼容的情況下重用包來節省本地存儲

Erg 的包管理器嚴格執行這些規則，并將拒絕違反這些規則的包更新
Erg 包管理器與版本控制系統(例如 git)一起使用，以檢測代碼差異并在發布包時驗證版本控制的正確性
具體來說，包管理器會查看 API 的類型。如果類型是舊版本的子類型，則認為更改是兼容的(請注意，這不是完整的驗證；類型兼容但語義上不兼容的重大更改是可能的，這是開發人員的工作來確定這一點)

此外，由于整個包存儲庫都在注冊表中注冊，即使是開發人員也無法在不通過包管理器的情況下更新包
此外，包可以被棄用但不能被刪除

### 附錄: 語義版本控制問題和對策

語義版本控制存在(至少)兩個已知問題
首先，語義版本控制可能過于嚴格
使用語義版本控制，單個不兼容的 API 更改會增加整個包的主要版本
發生這種情況時，諸如"我想嘗試一個新的 API，但我必須處理另一個不兼容的 API 更改，所以我不會升級"之類的事情
其次，語義版本控制可以承諾太多
如上一節所述，對 API 的"兼容更改"在理論上是不可證明的。如果您指定要使用版本為 `1.0.1` 的包，則可以在語義版本控制方面使用 `1.0.1` 和 `2.0.0` 之間的任何包(`1.0.0` 不能被使用，因為錯誤已被修復)，但由于包開發人員無意使用 API，構建可能不會成功

Erg 通過允許同時使用不同版本的包(通過重命名)解決了這個問題。這使得在部分引入 ver2 API 的同時繼續使用 ver1 API 成為可能
此外，雖然這不是一個非常理想的狀態，但如果只能使用 API 的某個次要版本而沒有錯誤，則可以不理會它并繼續前進到下一個版本

## 發布

可以使用 `publish` 子命令發布包。發布需要 GitHub 帳戶
默認情況下，包使用 `(owner_name)/(package_name)` 注冊。如果滿足一定條件(下載次數、維護頻率等)，可以申請注冊一個省略所有者名稱的別名
請注意，包名稱不區分大小寫，并且不區分諸如 `_` 和 `-` 之類的分隔符。