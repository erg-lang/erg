# 副作用和过程

到目前为止，我一直没有解释中<gtr=“11”/>的含义，现在我终于明白了它的含义。这个！直截了当地表示此对象是具有“副作用”的“过程”。过程对函数产生了一种称为“副作用”的效果。


```erg
f x = print! x # EffectError: functions cannot be assigned objects with side effects
# hint: change the name to 'f!'
```

上面的代码是编译错误。因为你在函数中使用过程。在这种情况下，必须将其定义为过程。


```erg
p! x = print! x
```

，<gtr=“13”/>，...是代表过程的典型变量名称。以这种方式定义的过程也不能在函数中使用，因此副作用是完全隔离的。

## 方法

每个函数和过程都有一个方法。函数方法仅保留的不变引用，过程方法保留<gtr=“15”/>的可变引用。<gtr=“16”/>是一个特殊参数，在方法上下文中是指调用的对象本身。引用的<gtr=“17”/>不能指定给任何其他变量。


```erg
C.
    method ref self =
        x = self # OwnershipError: cannot move out 'self'
        x
```

该方法还可以剥夺的所有权。该方法的定义不包括<gtr=“19”/>或<gtr=“20”/>。


```erg
n = 1
s = n.into(Str) # '1'
n # ValueError: n was moved by .into (line 2)
```

始终只能有一个过程方法具有可变引用。此外，当可变参照被取走时，将无法从原始对象获取参照。从这个意义上说，会对<gtr=“22”/>产生副作用。

但是，请注意，可以从可变参照生成（不变/可变）参照。这允许你在过程方法中递归或。


```erg
T -> T # OK (move)
T -> Ref T # OK
T => Ref! T # OK (only once)
Ref T -> T # NG
Ref T -> Ref T # OK
Ref T => Ref! T # NG
Ref! T -> T # NG
Ref! T -> Ref T # OK
Ref! T => Ref! T # OK
```

## Appendix：严格定义副作用

代码有没有副作用的规则并不是马上就能理解的。在理解之前，建议先将其定义为函数，然后在出现错误时将其定义为过程。但是，对于那些想要掌握语言严格规范的人来说，下面我们会更详细地介绍副作用。

首先，请注意，返回值的等价性与 Erg 中的副作用无关。对于任何，都有一个过程（例如，总是返回<gtr=“27”/>），也有一个函数是<gtr=“28”/>。

前一个示例是，后一个示例是以下函数。


```erg
nan _ = Float.NaN
assert nan(1) != nan(1)
```

也有一些对象无法进行等价判定，例如类或函数。


```erg
T = Structural {i = Int}
U = Structural {i = Int}
assert T == U

C = Class {i = Int}
D = Class {i = Int}
assert C == D # TypeError: cannot compare classes
```

回到正题上来。“副作用”在 Erg 中的确切定义是，

* 访问外部可变信息

中选择所需的墙类型。外部通常是指外部范围。“外部”不包括 Erg 无法接触的计算机资源或运行前/运行后信息。“访问”不仅包括写入，还包括读取。

以过程为例。<gtr=“31”/>看似没有重写任何变量。但是，如果这是一个函数，那么外部变量可以用这样的代码重写。


```erg
camera = import "some_camera_module"
ocr = import "some_ocr_module"

n = 0
_ =
    f x = print x # 仮にprintを関数として使えたとします
    f(3.141592)
cam = camera.new() # カメラはPCのディスプレイを向いています
image = cam.shot!()
n = ocr.read_num(image) # n = 3.141592
```

模块是为相机产品提供 API 的外部库，<gtr=“33”/>是 OCR（光学字符识别）的库。直接副作用是由<gtr=“34”/>引起的，但显然，这些信息是从<gtr=“35”/>泄露的。因此，<gtr=“36”/>在性质上不能是函数。

然而，当你在函数中临时检查值时，你可能不希望将附加到相关函数中。在这种情况下，可以使用函数<gtr=“38”/>。<gtr=“39”/>在执行整个代码后显示值。这不会传播副作用。


```erg
log "this will be printed after execution"
print! "this will be printed immediately"
# this will be printed immediately
# this will be printed after execution
```

换句话说，如果程序没有反馈，即任何外部对象都不能使用该信息，则信息的“泄露”本身可能是允许的。不被“传播”就行了。

<p align='center'>
    <a href='./06_operator.md'>Previous</a> | <a href='./08_procedure.md'>Next</a>
</p>
