set -euo pipefail

cd assets
RUST_LOG=debug RUST_BACKTRACE=1 cargo build
rm -rf ./memurycard.exe
cp ../target/debug/memurycard.exe .
./memurycard.exe
