set export := true

VERSION := "0.0.9"

BIN_PATH_RELEASE := "target/release/gamacrosd"
BIN_PATH_DEBUG := "target/debug/gamacrosd"

BREW_LIBRARY_PATH := `brew --prefix` / "lib"

export PKG_CONFIG_PATH := BREW_LIBRARY_PATH / "pkgconfig"
export LIBRARY_PATH := BREW_LIBRARY_PATH
export RUSTFLAGS := "-L native=" + BREW_LIBRARY_PATH + " -C link-args=-Wl,-rpath," + BREW_LIBRARY_PATH

clean:
  cargo clean

start *ARGS:
  cargo run -v -p gamacrosd -- {{ARGS}}

[group: 'build']
install:
  cargo install --path crates/gamacrosd

[group: 'build']
build: build-release

[group: 'build']
build-release:
  cargo build --release -p gamacrosd

[group: 'build']
build-debug:
  cargo build -p gamacrosd

# Quality Assurance
[group: 'qa']
lint:
  cargo clippy

[group: 'qa']
fix *ARGS:
  cargo clippy --fix --allow-dirty {{ARGS}}

[group: 'qa']
check-formatting:
  cargo fmt --all --check

[group: 'qa']
test *ARGS:
  cargo nextest run {{ARGS}}

[group: 'qa']
test-coverage:
  cargo llvm-cov nextest

[group: 'qa']
format:
  cargo fmt --all

qa: lint check-formatting test

# Memory testing
[group: 'mem']
mem-scenario duration='10':
  #!/usr/bin/env sh
  set -eo pipefail
  cargo build --release -p gamacrosd
  "$BIN_PATH_RELEASE" > /tmp/gamacrosd_mem.log 2>&1 &
  PID=$!
  echo "Started gamacrosd PID=$PID"
  # Give it a moment to initialize
  sleep 2
  # Baseline memory snapshot
  echo "baseline_rss_kb=$(ps -o rss= -p $PID | tr -d ' ')" \
    | tee /tmp/gamacrosd_mem_metrics.txt
  echo "Perform a couple of joystick events now (press 2 buttons, move a stick)" \
    | tee -a /tmp/gamacrosd_mem_metrics.txt
  # Allow time for interaction
  sleep {{duration}}
  # After-interaction snapshot
  echo "after_rss_kb=$(ps -o rss= -p $PID | tr -d ' ')" \
    | tee -a /tmp/gamacrosd_mem_metrics.txt
  # Optional deep summary if tools exist
  if command -v vmmap >/dev/null; then
    /usr/bin/vmmap -summary $PID | egrep 'Physical footprint|PhysFootprint' \
        | tee -a /tmp/gamacrosd_mem_metrics.txt || true
  fi
  if command -v leaks >/dev/null; then
    /usr/bin/leaks -quiet $PID | tail -n 10 | tee -a /tmp/gamacrosd_mem_metrics.txt || true
  fi
  # Graceful shutdown via SIGINT (same as Ctrl+C)
  kill -INT $PID || true
  wait $PID || true
  # Produce a tiny CSV for quick comparisons
  awk 'BEGIN{print "metric,value"} /rss_kb/{split($0,a,"="); print a[1]","a[2]}' \
    /tmp/gamacrosd_mem_metrics.txt > /tmp/gamacrosd_mem.csv
  echo "Saved metrics to /tmp/gamacrosd_mem_metrics.txt and /tmp/gamacrosd_mem.csv"

[group: 'mem']
mem-peak:
  #!/usr/bin/env sh
  set -euo pipefail
  cargo build --release -p gamacrosd
  echo "Running with /usr/bin/time -l (stop with Ctrl+C after exercising the joystick)"
  /usr/bin/time -l "$BIN_PATH_RELEASE"

[group: 'mem']
mem-xctrace duration='15':
  #!/usr/bin/env sh
  set -euo pipefail
  cargo build --release -p gamacrosd
  OUT="/tmp/gamacrosd_mem.trace"
  echo "Recording Instruments Memory Usage trace to $OUT for {{duration}}s..."
  xcrun xctrace record --template 'Memory Usage' --time-limit {{duration}}s --output "$OUT" --launch "$BIN_PATH_RELEASE"
  echo "Done. Open $OUT in Instruments to inspect allocations and footprint."

[group: 'release']
publish:
	@python3 ./scripts/update_version.py {{VERSION}}
	@cargo update -p gamacrosd
	@git add \
		justfile \
		Cargo.lock \
		crates/gamacrosd/Cargo.toml
	@git commit -m "chore: release v{{VERSION}} 🔥"
	@git tag v{{VERSION}}
	@git-cliff -o CHANGELOG.md
	@git tag -d v{{VERSION}}
	@git add CHANGELOG.md
	@git commit --amend --no-edit
	@git tag -a v{{VERSION}} -m "release v{{VERSION}}"
	@git push
	@git push --tags