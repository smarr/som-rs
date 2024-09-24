#!/bin/bash

for file in core-lib/TestSuite/*Test.som; do
    if [ -f "$file" ]; then
        test_name="${file:19:-8}"
        echo "$test_name"
        cargo run --bin ${EXE:=som-interpreter-ast} -- -c core-lib/Smalltalk core-lib/TestSuite -- TestHarness $test_name
    fi
done