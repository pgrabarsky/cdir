CDIR_BIN="__BIN_PATH__/cdir"

# New cd command that is:
# - calling the default cd command
# - call cdir to store the path
function cdir_cd {
    DIR="$*"
    if [ $# -eq 0 ]; then
        DIR=$HOME;
    fi;
    builtin cd "${DIR}"
    $CDIR_BIN add-path "`pwd`"
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
   $CDIR_BIN add-shortcut $1 "`pwd`"
}

# c command to change the current directory using shortcuts
function c {
    if [ $# -eq 0 ]; then
        TMP_FILE=`mktemp`
        $CDIR_BIN gui $TMP_FILE
        DIR="`cat $TMP_FILE`"
        [[ ! -z $DIR ]] && cd $DIR
        rm $TMP_FILE
    else
         cd "`$CDIR_BIN print-shortcut $1`"
    fi
}
