#!/usr/bin/env bash
# Runs the build's tests: quick (the default; build.sh runs these first), slow, or all.
#
# Usage: tests/run.sh [quick|slow|all] [unittest options, such as -v or -k NAME]
#
# THIMSY_TEST_CACHE=<the build's work/cache> adds the tests of the mods that
# need its downloads; they copy what they use and never write to it.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
mode="${1:-quick}"
case "$mode" in
  quick | slow | all) ;;
  *) echo "usage: tests/run.sh [quick|slow|all] [unittest options]" >&2; exit 2 ;;
esac
shift $(($# ? 1 : 0))
cd "$ROOT"
# The scripts under test leave files to the garbage collector when they exit early.
THIMSY_TEST_MODE="$mode" exec python3 -I -W ignore::ResourceWarning -m unittest discover -s tests -t "$ROOT" "$@"
