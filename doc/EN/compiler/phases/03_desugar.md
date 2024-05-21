# Desugaring

To prevent the processing from becoming bloated after type analysis, Erg desugars some syntactic sugars at the parsing stage.
A typical syntactic sugar is the pattern. All patterns are reduced to a combination of simple variable assignments and type specifications.
