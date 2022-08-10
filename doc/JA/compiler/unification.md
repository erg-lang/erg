# 単一化(Unification)

## 出現検査(Occur Check)

Ergは再帰型を許可するが、意味の無い、ナンセンスな再帰型はエラーとする。ナンセンスな型とは、具体的な型を挙げることができない型である。そのチェックするのが出現検査である。
下の例は、意味のある型である。

```erg
T = Int
T = Int or Option T # Int or Option Int or Option Option Int or ...
Maybe T = Option T
T = Int or T # will be warned (should just be `Int`)
```

対して以下は、意味をなさない型である。

```erg
T = T
T = Option T
T = T or T

T = U
U = T

T X = T X
U T = T U
```

判定のアルゴリズムは、大まかにはこうである。

* ある未判定の型`T`を判定する時、それが「定義済みの型を含むor型」でなく、`T`を含む場合はエラーとなる。
* 多項カインドが単純型として扱われた場合はエラーとなる。

注意として、Ergは`F = F -> T`(`T`は任意の型)型の存在を許す。
