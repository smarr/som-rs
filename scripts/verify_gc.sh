#!/bin/bash

BENCHMARKS=("Bounce" "Mandelbrot" "TreeSort" "List" "Permute" "Queens" "IntegerLoop" "QuickSort" "Sieve" "Fannkuch" "JsonSmall" "DeltaBlue" "Richards" "Towers")

GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

run_benchmarks() {
    local exe=$1
    for bench in "${BENCHMARKS[@]}"
    do
        local output="Running $bench ($exe): ..."

        export RUST_LOG=off
        cmd_output=$(cargo run --bin som-interpreter-$exe --features=som-gc/stress_test -- -c core-lib/Smalltalk core-lib/Examples/Benchmarks \
          core-lib/Examples/Benchmarks/LanguageFeatures core-lib/Examples/Benchmarks/Json core-lib/Examples/Benchmarks/Richards \
          core-lib/Examples/Benchmarks/DeltaBlue -- core-lib/Examples/Benchmarks/BenchmarkHarness.som $bench 2 15)

        if [ $? -eq 0 ]; then
            output+="${GREEN}OK${NC}"
        else
            output+="${RED}FAILED${NC}"
            echo -e "Output from failed benchmark:\n$cmd_output"
        fi

        echo -e "$output"
    done
}

run_benchmarks "ast" &
run_benchmarks "bc" &

wait
