#!/bin/sh
export LD_LIBRARY_PATH="$(dirname "$0")/BOINC/boinc-lib-output/lib:$LD_LIBRARY_PATH"
exec "$(dirname "$0")/target/debug/test_oracle" "$@"