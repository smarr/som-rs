#!/bin/bash

target_commit="76c3323cd7"

while [[ $(git rev-parse HEAD) != "$target_commit" ]]; do
  cargo --quiet run --bin som-interpreter-bc --features=inlining-disabled -- -c ./core-lib/Smalltalk ./core-lib/Examples/Benchmarks . -- BenchmarkHarness List 1 7
  git checkout HEAD~1
done

echo "Reached target commit $target_commit, exiting"
