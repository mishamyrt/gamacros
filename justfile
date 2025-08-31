[group: 'qa']
lint:
  cargo clippy

[group: 'qa']
check-formatting:
  cargo fmt --all --check

test:
  #!/bin/bash
  export LIBRARY_PATH="$LIBRARY_PATH:$(brew --prefix)/lib" 
  cargo nextest run

[group: 'qa']
collect-coverage:
  cargo llvm-cov nextest

qa: lint check-formatting test

format:
  cargo fmt --all
