# Troubleshooting

## Q: Local builds succeed, but GitHub Actions builds fail

A: The branch you are working on may not be following the changes in `main`.

## Q: The pre-commit check fails

A: Try committing again. It may fail the first time. If it fails again and again, the code may contain a bug.

## Q: pre-commit test gives "link failure"

A: Make sure cargo is not running in another process.

## Q: build.rs fails to run

A: Check for extra files/directories (such as `__pychache__`) on the directory where `build.rs` runs.
