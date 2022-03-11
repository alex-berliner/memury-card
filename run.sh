set -euo pipefail

if [[ $OSTYPE == "linux-gnu" ]]; then
    BINARY="memurycard"
    BINOPTS="-s settings_linux.json"
else
    BINARY="memurycard.exe"
    BINOPTS=""
fi

cd assets
RUST_LOG=debug RUST_BACKTRACE=1 cargo build
rm -rf ./$BINARY
cp ../target/debug/$BINARY .

if [[ $1 == "run" ]]; then
    echo $BINARY $BINOPTS
    ./$BINARY $BINOPTS
fi
