# Simple_AUR_Helper
a simple aur helper with test used to manage downloaded aur packages in a directory

## Usage

General structure: `aur_helper [AUR_DIR] <-command> [params]`

-> structured like pacman

> NOTE: AUR_DIR defaults to $HOME/AUR/ 


## Features 
### Implemented

-   search single or multiple packages
-   update (git pull) all dirs and further actions with successful ones(build, update)
-   check, which packages are installed and which are orphaned
-   build all packages in the dir and further actions with successful ones (install)
-   install all latest builds again

### Todo

-   download with -Sd
-   maybe install with -Sdi
-   shell completion (bash, zsh)
-   gen manage
-   update, build and install for specific package
