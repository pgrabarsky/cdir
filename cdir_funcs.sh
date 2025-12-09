# New cd command that is:
# - calling the default cd command
# - call cdir to store the path
function cdir_cd {
    DIR="$*"
    if [ $# -eq 0 ]; then
        DIR=$HOME;
    fi;
    builtin cd "${DIR}"
    cdir add-path "`pwd`"
}

# Mimic default auto-complete of the default cd command
_cdir_cd () {
  ((CURRENT == 2)) &&
  _files -/ -W ${PWD}
}

# Use cdir_cd as the new cd command
alias cd="cdir_cd"

# Create a new shortcut in cdir
function p {
   cdir add-shortcut $1 "`pwd`"
}

# c command to change the current directory using shortcuts
function c {
    if [ $# -eq 0 ]; then
        TMP_FILE=`mktemp`
        cdir gui $TMP_FILE
        DIR="`cat $TMP_FILE`"
        [[ ! -z $DIR ]] && cd $DIR
        rm $TMP_FILE
    else
         cd "`cdir print-shortcut $1`"
    fi
}
