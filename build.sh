if [ -f "./Cargo.toml" ]; then
    cargo build --release
    cargo build --release --target armv7-unknown-linux-gnueabihf
else
    echo "Cargo.toml not found. Please run this script from the root of your Rust project."
    exit 1
fi

ARM64=./target/release/slideshowsetuprust
ARM32=./target/armv7-unknown-linux-gnueabihf/release/slideshowsetuprust

if [ -f "$ARM64" ]; then
    echo "Copying ARM64 binary to current directory..."
    cp "$ARM64" ./slideshowsetuprust-rpi4-"$1"
else
    echo "ARM64 binary not found. Please ensure it was built successfully."
fi

if [ -f "$ARM32" ]; then
    echo "Copying ARM32 binary to current directory..."
    cp "$ARM32" ./slideshowsetuprust-rpi3b-"$1"
else
    echo "ARM32 binary not found. Please ensure it was built successfully."
fi

