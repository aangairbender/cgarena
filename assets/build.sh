# Argument $1 is bot's directory.
# Bot's source code is "$1"/source.txt
# Argument $2 is bot's language.

if [ "$2" = "c++" ]; then
  g++ -std=c++20 -x c++ "$1"/source.txt -o "$1"/a
elif [ "$2" = "python" ]; then
  cp "$1"/source.txt "$1"/a.py
# you would need "rust-workdir" with empty rust project and proper cargo.toml
# elif [ "$2" = "rust" ]; then
#   cp "$1"/source.txt rust-workdir/src/main.rs
#   cd rust-workdir
#   cargo build --release
#   cd ..
#   mv rust-workdir/target/release/rust-workdir* "$1"/
else
  echo "Unsupported language '$2'" >&2
  exit 1
fi