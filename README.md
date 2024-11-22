# NVME Driver

nvme driver 1.4

## example run

install qemu.

```shell
cargo install ostool
./img.sh

# run test with qemu
cargo test --test tests --  --show-output

# run test with real hardware that has uboot
cargo test --test tests --  --show-output --uboot
```
