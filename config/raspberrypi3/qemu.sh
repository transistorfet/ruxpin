
#KERNEL=target/aarch64-unknown-none/debug/ruxpin
#KERNEL=target/aarch64-unknown-none/release/ruxpin
KERNEL=ruxpin.img

#MMC_IMAGE=../ext2-disk-image.img
MMC_IMAGE=../../ruxpin-ext2-image.bin

qemu-system-aarch64 \
	-machine raspi3b -m 1024 \
	-kernel "$KERNEL" \
	-no-reboot -gdb tcp::1234 \
	-drive format=raw,if=sd,file=$MMC_IMAGE \
	-serial stdio
	#-d "int" \
	#-serial stdio -monitor tcp:localhost:1235 -S
	#-chardev stdio,mux=on,id=char0 -monitor chardev:char0 -S
	#-chardev stdio,mux=on,id=char0 -serial chardev:char0 -monitor chardev:char0 -S
	# -append "root=/dev/sda2 panic=1 rootfstype=ext4 rw init=/bin/bash" -hda rpi.img
