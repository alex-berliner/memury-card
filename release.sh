cargo build --release
rm -rf target/memurycard
mkdir -p target/memurycard
cp ./target/release/savedir.exe target/memurycard/memurycard.exe
cp -r tracker/ target/memurycard
sed -u 's/Code\\\\savesync/Dropbox/g' settings.json > target/memurycard/settings.json
cp settings.json target/memurycard/settings.json
