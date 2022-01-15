set -euo pipefail

if [[ $OSTYPE == "linux-gnu" ]]; then
    BINARY="memurycard"
else
    BINARY="memurycard.exe"
fi

cd assets
RUST_LOG=debug RUST_BACKTRACE=1 cargo build
rm -rf ./$BINARY
cp ../target/debug/$BINARY .

if [[ $1 == "run" ]]; then
    ./$BINARY
fi
