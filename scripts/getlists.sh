#!/bin/bash
# Fetches lists of servers to block. Initially the list was the same as used by PiHole
set -x

# break on errors
set -e
# break on errors in pipes, e.g. a | b | c
set -o pipefail

DEBUG=$1
if [[ "${DEBUG}" == "debug" ]]; then
  echo "Debug mode, will fetch each source in separate files."
else
  OUT=concatenated.list
  rm -f $OUT
  echo "Building $OUT"
fi

# Extracts the URL from a line by removing # comments and trimming
# $1 - line
# return - global $LIST
function extract_url {
  # Determine if 'extglob' is currently on.
  local extglobWasOff=1
  shopt extglob >/dev/null && extglobWasOff=0
  (( extglobWasOff )) && shopt -s extglob # Turn 'extglob' on, if currently turned off.
  local var=$1
  # Remove commment
  var=${var%%#*}
  # Trim leading and trailing whitespace
  var=${var##+([[:space:]])}
  var=${var%%+([[:space:]])}
  (( extglobWasOff )) && shopt -u extglob # If 'extglob' was off before, turn it back off.
  #echo -n "$var"  # Output trimmed string.
  LIST=$var
}

# Creates a file name out of the URL to save the content for debug
# removes the https:// and replaces /, &, ? with _
# $1 - url
# return - global $OUT
function make_file_name {
  local F=${1/#https:\/\//}
  F=${F//\//_}
  F=${F//&/_}
  F=${F//\?/_}
  OUT=${F/%.txt/}.hosts
}

# Fetches the content from a list of lists
# $1 file name of list of lists
# result - if debug, individual files per list, else $OUT
function fetch_list_of_lists {
  while read f
  do
    extract_url "$f"
    if [[ ! -z "$LIST" ]]; then
      echo "Fetching: $LIST"
      if [[ "${DEBUG}" == "debug" ]]; then
        make_file_name $f    
        rm -f ${OUT}
      else
        cat >> $OUT <<FILEHEADER

#----------------------------------------------------- 
# dns-block: $LIST
#-----------------------------------------------------

FILEHEADER
      fi
      curl --insecure --fail --max-time 10 --retry 10 --retry-delay 0 "$LIST" >> "${OUT}"
    fi
  done < $1
}

fetch_list_of_lists list_of_lists.txt
fetch_list_of_lists own_list_of_lists.txt

if [[ ! "${DEBUG}" == "debug" ]]; then
  ./dns-block -dd $OUT domains.whitelisted hosts_blocked.txt domains.blocked
  ./dns-block -dd --bind $OUT domains.whitelisted hosts_blocked.txt rpz.db
fi
