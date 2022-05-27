
TARGETDIR = target/aarch64-unknown-none/release
COREUTILS = ls args cat ps rm mv mkdir
WORKSPACE_MEMBERS = bin/coreutils bin/sh config/raspberrypi3 kernel lib/api lib/app lib/syscall_proc


MOUNTPOINT = build
IMAGE = ruxpin-ext2-image.bin
BLOCKSIZE = 4096
IMAGE_BLOCKS = 1048576		# 4GiB
PARTITION_OFFSET = 272629760	# Partition 2: 0x8200 * 512
PARTITION_BLOCKS = 982016
LOOPBACK = /dev/loop8

COREUTILS_OUTPUTS = $(foreach CMD, $(COREUTILS), bin/coreutils/$(TARGETDIR)/$(CMD))

all: build-kernel


create-image:
	dd if=/dev/zero of=$(IMAGE) bs=4K count=$(IMAGE_BLOCKS)
	dd if=partition-table.bin of=ruxpin-ext2-image.bin bs=512 count=1 conv=notrunc
	sudo losetup --offset $(PARTITION_OFFSET) $(LOOPBACK) $(IMAGE)
	sudo mkfs.ext2 -b $(BLOCKSIZE) $(LOOPBACK) $(PARTITION_BLOCKS)
	sudo losetup -d $(LOOPBACK)

mount-image:
	sudo losetup --offset $(PARTITION_OFFSET) $(LOOPBACK) $(IMAGE)
	sudo mount -t ext2 $(LOOPBACK) $(MOUNTPOINT)

umount-image:
	- sudo umount $(MOUNTPOINT)
	sudo losetup -d $(LOOPBACK)

load-image:
	make mount-image
	- make load-image-contents
	make umount-image

coreutils:
	cd bin/$@ && cargo build --release && cd ../../ && rust-strip $(COREUTILS_OUTPUTS)

sh:
	cd bin/$@ && cargo build --release && rust-strip $(TARGETDIR)/$@

load-image-contents: sh coreutils
	sudo mkdir -p $(MOUNTPOINT)/bin
	sudo cp bin/sh/$(TARGETDIR)/sh $(MOUNTPOINT)/bin
	sudo cp $(COREUTILS_OUTPUTS) $(MOUNTPOINT)/bin


build-kernel:
	cd kernel && make

clean:
	rm -rf $(foreach member, $(WORKSPACE_MEMBERS), $(member)/target)


