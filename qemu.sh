
qemu-system-aarch64 \
	-machine raspi3b -m 1024 \
	-kernel target/aarch64-unknown-none/debug/ruxpin \
	-no-reboot -gdb tcp::1234 \
	-serial stdio
	#-serial stdio -monitor tcp:localhost:1235 -S
	#-chardev stdio,mux=on,id=char0 -monitor chardev:char0 -S
	#-chardev stdio,mux=on,id=char0 -serial chardev:char0 -monitor chardev:char0 -S
	# -append "root=/dev/sda2 panic=1 rootfstype=ext4 rw init=/bin/bash" -hda rpi.img
