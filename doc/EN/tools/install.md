# install subcommand

You can install packages registered on the registry site with install.
The basic usage is the same as package managers such as cargo.

## Convenience functions

* If there is a package name with a similar name and the number of downloads is more than 10 times that of that one, a suggestion will appear that you may have entered it incorrectly. This prevents typo squatting.
* If the package size is large (more than 50MB), display the size and suggest if you really want to install it.
* Suggest an alternative package if the package is duplicated.