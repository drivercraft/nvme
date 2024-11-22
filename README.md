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
# Connect to the board with serial, insert the Ethernet cable into the board 
# so that the host and the board are on the same network segment.
cargo test --test tests --  --show-output --uboot
```
