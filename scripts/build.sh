mkdir -p esp/efi/boot

cargo build --bin=loader --target=x86_64-unknown-uefi
cp target/x86_64-unknown-uefi/debug/loader.efi esp/efi/boot/bootx64.efi

cargo build --bin=kernel --target=x86_64-unknown-none
cp target/x86_64-unknown-none/debug/kernel esp/kernel.elf
