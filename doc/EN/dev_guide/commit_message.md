# Guidelines for Commit Messages(PR titles use the same method)

This guideline is intended to

* Clarify how commit messages should be written
* Make it easier to refer to the commit message later

and so on. It is an effort goal, and does not require you to rebase and modify your commit message if it deviates from the format (you can use the `--amend` option if you want to change the previous commit message).

There are two rules you should follow:

* Automatically-added commit messages (e.g. `Update xxx.md`, `Automatic update xxx`, `Merge pull request #XXX ...`) should be sent as they are, without modification.
* Manual commit messages should conform to [conventional commits](https://www.conventionalcommits.org/en/v1.0.0/#specification)

BNF for conventional commits is as follows.

```bnf
commit ::= type ('('' scope ')')? '!' ? ':' description body? footer*.
type ::= 'feat' | 'fix' | 'docs' | 'style' | 'refactor' | 'perf' | 'test' | 'build' | 'ci' | 'chore' | 'revert'
```

Since we develop on GitHub, we'll extend this a bit and allow issue/PR numbers to be added after the description.

```bnf
commit ::= type ('('' scope ')')? '!' ? ':' description ('(' '#' issue ')')? body?
```

The meaning of each part is as follows.

* `type` indicates the type of commit. Please write it in lower case (automatic commits start with a capital letter, so this distinguishes whether it is a manual commit or not). `feat` is a new feature, `fix` is a bug fix or issue resolution, `docs` is a change in documentation, `style` is a change in code style, `refactor` is a refactoring, `perf` is performance improvement, `test` is adding or changing tests, `build` is build-related/version/dependency changes, `ci` is CI-related changes, `chore` is internal/minor changes, and `revert` is revert. If more than one applies, please select the more specific type. The lower priority types are `fix`, `refactor`, `style`, and `chore`. For example, `docs` is for fixing (fix) documentation (docs) and `test` is for refactoring (refactor) tests (test).

* `scope` is optional and indicates the scope of the commit. For example, the commit message `fix(parser):` indicates a bug fix for the parser. You may specify multiple scopes separated by commas, but in that case you should also consider splitting the commit. Examples of scopes are:

  * `parser`
  * `compiler`
  * `els`

* The `!` mark indicates that the commit has destructive changes. If this mark is set, the reason for the destructive change must be written. Destructive changes include language specification changes, compiler API changes, and so on.

* `description` is a summary of the commit. It should not be too short, but should be approximately 50 characters or less. Basically it should be written in English. Do not begin with a lowercase letter unless it begins with an uppercase word. Do not include a period.

* `body` is optional and indicates the details of the commit.

* `footer` is optional and represents information related to the commit (e.g. list of reviewers, related issue/PR numbers, links, etc.).

---

Here are examples:

```txt
feat(parser): add support for foo (#123)
```

```txt
fix: address CVE-XXXX-YYYY

Ref: https://cve.mitre.org/...
```

```txt
docs!: remove `xxx.md`

The contents of xxx.md are old and inaccurate, so it is deleted.
```

```txt
docs: update commit hash of `xxx.md`
```

```txt
refactor(compiler): `Foo` => `FooBar`
```

```txt
build: update version (v0.1.2 => v0.1.3)
```

```txt
style: fix typo
```

As you can see from the examples, API and file/directory names should be enclosed in ``.

## Supplemental

* You are free to write commits in the middle of your work. When you finally squash and organize your work, please follow the rules.
* Basically use the present and ongoing tenses for sentences.
* If there are messy commits in PR, please use squash and merge(If the commit is clear, merge directly)
