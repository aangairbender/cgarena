# Argument $1 is bot's directory.
# Bot's source code is "$1"/source.txt
# Argument $2 is bot's language.

if [ "$2" = "c++" ]; then
  ./"$1"/a
elif [ "$2" = "python" ]; then
  python ./"$1"/a.py
# you would need "rust-workdir" with empty rust project and proper cargo.toml
# elif [ "$2" = "rust" ]; then
#   ./"$1"/rust-workdir
else
  echo "Unsupported language '$2'" >&2
  exit 1
fi