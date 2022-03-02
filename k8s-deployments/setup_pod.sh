apt-get update
apt-get install -y curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh  -s -- -y
source $HOME/.cargo/env
apt-get install -y \
    curl gcc g++ git jq make pkg-config cmake \
    libclang1 libssl-dev llvm-dev