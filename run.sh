set -euo pipefail

cd assets
RUST_LOG=debug RUST_BACKTRACE=1 cargo build
rm -rf ./savedir.exe
cp ../target/debug/savedir.exe .
./savedir.exe
