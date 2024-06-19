#!/bin/bash

# Directory to process
DIRECTORY="core-lib/TestSuite" # Replace with your directory path
SOM_RS_EXE=${SOM_RS_EXE:-som-interpreter-bc}

for file in $DIRECTORY/*Test.som; do
    if [ -f "$file" ]; then
        test_name="${file:19:-8}"
        echo "$test_name"
        cargo run --bin $SOM_RS_EXE -- -c core-lib/Smalltalk core-lib/TestSuite -- TestHarness $test_name
    fi
done