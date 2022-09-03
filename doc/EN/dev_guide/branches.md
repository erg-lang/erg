# Branch naming and operation policy

* Basically, development is done in one `main` branch (monorepo development). Create a `feature-*` branch or `issue-*` branch only when it is difficult to work without branching.

## main

* main development branch
* The following conditions must be met

* Compile succeeds

## beta (not created at this time)

* Latest beta release
* The following conditions must be met

* Compile succeeds
* All tests passed

## feature-(name)

* A branch that develops one specific feature

* No conditions

## issue-(#issue)

* branch that resolves a specific issue

* No condition

## fix-(#issue or bug name)

* branch that fixes a specific bug (If the issue is a bug, please create this instead of `issue-*`)

* No condition
