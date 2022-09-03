# NonZeroN

A class that represents a non-zero number. The safety of division by zero is guaranteed.

```mermaid
class Diagram
     class NonZero~Int~ {
         ...
     }
     class Int {
         ...
     }
     class Div {
         <<trait>>
         /(Self, R) -> O or Panic
     }
     class SafeDiv {
         <<trait>>
         /(Self, R) -> O
     }
     Int <|-- NonZero~Int~: Inherit
     Div <|-- SafeDiv: Subsume
     SafeDiv <|..NonZero~Int~: Impl
     Div <|.. Int: Impl
```

## methods

@Impl SafeDiv R, O
.`/`: Self.(R) -> O