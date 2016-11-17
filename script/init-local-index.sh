#!/bin/sh

set -e

if [ -d tmp/index-bare ]; then
    echo tmp/index-bare already exists, exiting
    exit 0
fi

mkdir -p tmp
rm -rf tmp/index-bare tmp/index-co

echo "Initializing repository in tmp/index-bare..."
git init -q --bare tmp/index-bare

echo "Creating checkout in tmp/index-bare..."
git init -q tmp/index-co
cd tmp/index-co
cat > config.json <<-EOF
{
  "dl": "http://localhost:8888/api/v1/crates",
  "api": "http://localhost:8888/"
}
EOF
git add config.json
git commit -qm 'Initial commit'
git remote add origin file://`pwd`/../index-bare
git push -q origin master -u > /dev/null
cd ../..
touch tmp/index-co/.git/git-daemon-export-ok

cat - <<-EOF
Your local git index is ready to go!

You'll want to build crates.io by running:

   cargo build

Follow up by changing your HOME/.cargo/config: 

   [source]

   [source.local]
   registry = "https://localhost:8888/crates.io-index"

   [source.crates-io]
   replace-with = "local"
   registry = 'https://doesnt-matter-but-must-be-present'

You will also need to generate a token from in the app itself

Please refer to https://github.com/rust-lang/crates.io/blob/master/README.md for more info!
EOF
