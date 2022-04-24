
TARGETDIR = target/aarch64-unknown-none/release


MOUNTPOINT = build
IMAGE = ruxpin-ext2-image.bin
BLOCKSIZE = 4096
IMAGE_BLOCKS = 1048576		# 4GiB
PARTITION_OFFSET = 272629760	# Partition 2: 0x8200 * 512
PARTITION_BLOCKS = 982016
LOOPBACK = /dev/loop8


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
	cd bin/coreutils && cargo build --release

testapp:
	cd bin/testapp && cargo build --release

sh:
	cd bin/sh && cargo build --release

load-image-contents: testapp sh coreutils
	sudo mkdir -p $(MOUNTPOINT)/bin
	sudo cp bin/testapp/$(TARGETDIR)/testapp $(MOUNTPOINT)/bin
	sudo cp bin/sh/$(TARGETDIR)/sh $(MOUNTPOINT)/bin
	sudo cp bin/coreutils/$(TARGETDIR)/ls $(MOUNTPOINT)/bin
	sudo cp bin/coreutils/$(TARGETDIR)/args $(MOUNTPOINT)/bin
	sudo cp bin/coreutils/$(TARGETDIR)/cat $(MOUNTPOINT)/bin


build-kernel:
	cd kernel && make


