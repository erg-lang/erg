# alstructuration
Modules qui fournissent des caractères représentant les structures et les parcelles d'algèbre.

- membres membres membres membres membres

## binop
    BinOp Op: Kind 2 = Subsume Op(Self, Self.ReturnTypeOf Op), Additional: {
        .ReturnTypeof = TraitType -&gt; Type
    }
    
    Nat &lt;: BinOp Add
    assert Nat. ReturnTypeof(Add) == Nat
    assert Nat. ReturnTypeof(Sub) == Int
    assert Nat. ReturnTypeof(Mul) == Nat
    assert Nat.ReturnTypeof(Div) == Positive Ratio

## semi-groupe
    SemiGroup Op: Kind 2 = Op(Self, Self)
    
    IntIsSemiGroupAdd = Patch Int, Impl=SemiGroupAdd
    
    Int &lt;: SemiGroup Add

## amateurs
    ## * Identity law: x.map(id) == x
    ## * Composition law: x.map(f).map(g) == x.map(f.then g)
    Functor = Trait {
        .map|T, U: Type| = (Self(T), T -&gt; U) -&gt; Self U
    }

## Application
    ## * Identity law: x.app(X.pure(id)) == x
    Applicative = Subsume Functor, Additional: {
        .pure|T: Type| = T -&gt; Self T
        .app|T, U: Type| = (Self(T), Self(T -&gt; U)) -&gt; Self U
    }

## monad monad
    Monad = Subsume Applicative, Additional: {
        .bind|T, U: Type| = (Self(T), T -&gt; Self U) -&gt; Self U
    }

