qemu-system-aarch64 \
	-machine virt,dumpdtb=target/qemu.dtb \
	-display none \
	-cpu cortex-a53 \
	-smp 1 \
	-drive file=target/nvme.img,if=none,id=nvm \
	-device nvme,serial=deadbeef,drive=nvm
dtc -I dtb -O dts -o target/qemu.dts target/qemu.dtb