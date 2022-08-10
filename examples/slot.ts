const assert = console.assert

const john = {
    name: "John Smith",
    age: 27,
}

assert(john.name == "John Smith")

type Person = {
    readonly name: String,
    readonly age: number,
}

// assert(Person['age'] == String)

function isPerson(arg): arg is Person { return true }

assert(isPerson(john))
