mkdir -p esp/efi/boot

if [ "$1" = "release" ]; then
    cargo build --bin=loader --target=x86_64-unknown-uefi --release
    cp target/x86_64-unknown-uefi/release/loader.efi esp/efi/boot/bootx64.efi
    cargo build --bin=kernel --target=x86_64-unknown-none --release
    cp target/x86_64-unknown-none/release/kernel esp/kernel.elf
else
    cargo build --bin=loader --target=x86_64-unknown-uefi
    cp target/x86_64-unknown-uefi/debug/loader.efi esp/efi/boot/bootx64.efi
    cargo build --bin=kernel --target=x86_64-unknown-none
    cp target/x86_64-unknown-none/debug/kernel esp/kernel.elf
fi
