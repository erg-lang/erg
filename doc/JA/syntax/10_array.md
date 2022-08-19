# 配列

配列はもっとも基本的な __コレクション(集約)__ です。
コレクションとは、内部にオブジェクトを複数保持できるオブジェクトのことです。

```erg
a = [1, 2, 3]
a: [Int; 3] # 型指定: セミコロンの後の数字は要素数
# 要素数がわからない場合は省略可能
a: [Int]

mut_a = [!1, !2, !3]
mut_a[0].inc!()
assert mut_a == [2, 2, 3]
```

配列には、原則として違う型のオブジェクトを入れることはできません。

```erg
[1, "a"] # TypeError: 1st element is Int, but 2nd element is Str
```

しかし、このように明示的に型指定すると制限を回避できます。

```erg
[1, "a"]: [Int or Str]
```

## スライス

配列は、複数の値をまとめて取り出すこともできます。これをスライスと呼びます。

```erg
l = [1, 2, 3, 4]
# Pythonのl[1:3]に相当
assert l[1..<3] == [2, 3]
assert l[1..2] == [2, 3]
# l[1]と同じ
assert l[1..1] == [2]
# Pythonのl[::2]に相当
assert l[..].step(2) == [2, 4]
```

スライスで得られるオブジェクトは配列の(不変)参照です。

```erg
print! Typeof l[1..2] # Ref [Int; 4]
```

<p align='center'>
    <a href='./09_builtin_procs.md'>Previous</a> | <a href='./11_dict.md'>Next</a>
</p>
