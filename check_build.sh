#!/bin/bash
cd /Users/r/git/wrt2
cargo check --workspace 2>&1 | grep -E "(error\[E[0-9]+\]|error:|warning:)" | head -100