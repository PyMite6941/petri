OS=$(uname -s)
case "$OS" in
  Linux*)     machine=Linux;;
  Darwin*)    machine=Mac;;
  CYGWIN*, MINGW*)    machine=Windows;;
  *)          machine="UNKNOWN:$OS"
esac

if [ "$machine" = "Linux" ]; then
  docker build -t run-system ./run-system
  docker run --rm -it run-system
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  source $HOME/.cargo/env
  rustup target add wasm32-unknown-unknown
elif [ "$machine" = "Mac" ]; then
  docker build -t run-system ./run-system
  docker run --rm -it run-system
elif [ "$machine" = "Windows" ]; then
  docker build -t run-system ./run-system
  docker run --rm -it run-system
else
  echo "Unsupported OS: $OS"
  exit 1
fi