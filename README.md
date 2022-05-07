
RuxpinOS
========

###### *Started March 1, 2022*

An attempt at making an Operating System in Rust for a Raspberry Pi 3.  Due to
time constraints, I've prioritized getting something that works instead of
trying new ideas, so for now it's a monolithic OS with a Unix-like API, but I
might change that in the future.  I largely copied what I had done for
[ComputieOS](https://jabberwocky.ca/projects/computie/) which is written in C.

At the moment, it has support for virtual memory, with on-demand page
allocation, but no support yet for swapping memory to disk.  It has a virtual
file system with support for the ext2 file system, as well as some in-memory
file systems.  It supports multiple processes with context switching triggered
by the system timer, but doesn't yet support multiple threads.

Currently, there is only a console driver (tty subsystem) and sd/emmc card
driver (block device subsystem).  The block driver subsystem provides a
bufcache to cache blocks read from disk by the file system.  Blocks that are
borrowed as mutable are marked dirty and will be written back in the next block
commit.  The ext2 writing support isn't well tested yet, so committing is
disabled for now.

Applications can be written in Rust and compiled with the included libraries,
which use the Aarch64 `SVC` instruction to perform system calls to the OS.  It
currently supports basic file operations, as well as `exit`, `fork`, and
`exec`.  The OS can directly load the elf binaries produced by cargo as
applications, and run them.  A simple shell program (launched by the kernel
after initialization) and `ls` command are available, but are only at a proof
of concept stage.


Compiling
---------

The OS consists of a kernel and a few applications which are all compiled
separately.  The applications can be loaded in to an ext2 partition, which can
be read by the kernel, but the kernel needs to be loaded separately, so it's
not included in the image.

Normally, when a raspberry pi boots, the firmware looks for a FAT partition on
the microSD card containing the file `kernel8.img`, which is then loaded at
address `0x80000` and run.  Once the kernel is running, it can then mount any
other partitions to use as a root file system.  When running in Qemu, the
kernel image is passed on the command line, along with the filename of a disk
image that contains the ext2 partition.

A Makefile is provided in the project root to build the ext2 disk image.  To
make it easier to test the code in qemu or on hardware, it creates a disk image
that looks the same as the microSD card (ie. a FAT partition plus an ext2
partition).  When using qemu, the FAT partition is ignored.

To create an image, from a linux computer run:

```sh
make create-image
make load-image
```

This will create a new 4GB image file, use `mkfs.ext2` to create a new file
system inside of it, mount it as a loopback device at `<project>/build`, and
then compile and load the applications into it.  It also copies a hard-coded
partition table into the image, which replicates the partitions used by the
hardware.

Once the image has been created, the kernel can be compiled and run in qemu,
using:

```sh
cd config/raspberrypi3/
make
./qemu.sh
```

To run in on a raspberry pi currently requires a USB serial console.  I'm using
the MiniLoad program from the [Rust Raspberry Pi OS
Tutorial](https://github.com/rust-embedded/rust-raspberrypi-OS-tutorials/) to
chainload the kernel over the serial port.  It might be possible to put the
compiled kernel image (`ruxpin.img`) into the boot partition of the created
file and write it to disk, but I haven't tested that yet.


Example Output
--------------

The following output was copied from the console when run in qemu.  It has a
lot of debug messages to show what's going on.  It starts by setting up the
kernel heap and page memory, registers the filesystem types, initialize the
device drivers, and mounts the root ext2 partition.  It then performs a number
of tests to check basic file system functionality, followed by launching the
first process (the shell).  A command is then typed in, shown between "<>",
which launches the `ls` program, prints a list of the files and directories in
`/`, and then exits back to the shell prompt.

```
starting kernel...
kernel heap: using 0x200000, size 14MiB
virtual memory: using region at PhysicalAddress(0x1000000), size 240 MiB, pages 61438
interrupts: initializing generic arm interrupt controller
fs: registering filesystem tmpfs
fs: registering filesystem devfs
fs: registering filesystem ext2
console: initializing
sd: initializing
sd: found partition 0 at 2000, 256 MiB
sd: found partition 1 at 82000, 740 MiB
fs: mounting ext2 at /, device Some(DeviceID(0, 2))
ext2: magic number ef53, block size 4096
ext2: total blocks 982016, total inodes 245760, unallocated blocks: 963991, unallocated inodes: 245742
ext2: features compat: 38, ro: 3, incompat: 2
ext2: allocating inode 13
fs: mounting devfs at /dev, device None
ext2: looking for "dev", found inode 13

Running some hardcoded tests before completing the startup

Mounting the tmpfs filesystem (simple in-memory file system)
ext2: allocating inode 14
fs: mounting tmpfs at /tmp, device None
ext2: looking for "tmp", found inode 14

Creating a directory and a file inside of it
ext2: allocating inode 15
ext2: looking for "testdir", found inode 15
ext2: allocating inode 16
ext2: allocating block 761 in group 0
ext2: allocating block 762 in group 0
ext2: writing to block 762
Read file 14: This is a test

Opening the console device file and writing to it
ext2: looking for "dev", found inode 13
the device file can write

Opening the testapp binary through the vfs interface and reading some data
ext2: looking for "bin", found inode 32769
ext2: looking for "testapp", found inode 32770
read in 1024 bytes
0xffff00000007f790: 7f 45 4c 46 02 01 01 00 00 00 00 00 00 00 00 00 
0xffff00000007f7a0: 02 00 b7 00 01 00 00 00 70 29 21 00 00 00 00 00 
0xffff00000007f7b0: 40 00 00 00 00 00 00 00 d0 c8 0d 00 00 00 00 00 
0xffff00000007f7c0: 00 00 00 00 40 00 38 00 04 00 40 00 10 00 0e 00 
0xffff00000007f7d0: 06 00 00 00 04 00 00 00 40 00 00 00 00 00 00 00 
0xffff00000007f7e0: 40 00 20 00 00 00 00 00 40 00 20 00 00 00 00 00 
0xffff00000007f7f0: e0 00 00 00 00 00 00 00 e0 00 00 00 00 00 00 00 
0xffff00000007f800: 08 00 00 00 00 00 00 00 01 00 00 00 04 00 00 00 
0xffff00000007f810: 00 00 00 00 00 00 00 00 00 00 20 00 00 00 00 00 
0xffff00000007f820: 00 00 20 00 00 00 00 00 60 09 00 00 00 00 00 00 
0xffff00000007f830: 60 09 00 00 00 00 00 00 00 00 01 00 00 00 00 00 
0xffff00000007f840: 01 00 00 00 05 00 00 00 60 09 00 00 00 00 00 00 
0xffff00000007f850: 60 09 21 00 00 00 00 00 60 09 21 00 00 00 00 00 
0xffff00000007f860: d8 31 00 00 00 00 00 00 d8 31 00 00 00 00 00 00 
0xffff00000007f870: 00 00 01 00 00 00 00 00 51 e5 74 64 06 00 00 00 
0xffff00000007f880: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 
0xffff00000007f890: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 
0xffff00000007f8a0: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 
0xffff00000007f8b0: 63 61 6c 6c 65 64 20 60 52 65 73 75 6c 74 3a 3a 
0xffff00000007f8c0: 75 6e 77 72 61 70 28 29 60 20 6f 6e 20 61 6e 20 
0xffff00000007f8d0: 60 45 72 72 60 20 76 61 6c 75 65 00 00 00 00 00 
0xffff00000007f8e0: 60 09 21 00 00 00 00 00 08 00 00 00 00 00 00 00 
0xffff00000007f8f0: 08 00 00 00 00 00 00 00 20 10 21 00 00 00 00 00 
0xffff00000007f900: 60 09 21 00 00 00 00 00 00 00 00 00 00 00 00 00 
0xffff00000007f910: 01 00 00 00 00 00 00 00 94 28 21 00 00 00 00 00 
0xffff00000007f920: 61 20 72 65 61 6c 6c 79 20 63 6f 6f 6c 20 6d 65 
0xffff00000007f930: 73 73 61 67 65 20 74 68 61 74 20 49 27 64 20 6c 
0xffff00000007f940: 69 6b 65 20 74 6f 20 73 65 65 00 00 00 00 00 00 
0xffff00000007f950: 90 01 20 00 00 00 00 00 2a 00 00 00 00 00 00 00 
0xffff00000007f960: 73 72 63 2f 6d 61 69 6e 2e 72 73 00 00 00 00 00 
0xffff00000007f970: d0 01 20 00 00 00 00 00 0b 00 00 00 00 00 00 00 
0xffff00000007f980: 0c 00 00 00 05 00 00 00 0a 2f 6d 6e 74 2f 74 65 
0xffff00000007f990: 73 74 32 00 00 00 00 00 d0 01 20 00 00 00 00 00 
0xffff00000007f9a0: 0b 00 00 00 00 00 00 00 0e 00 00 00 51 00 00 00 
0xffff00000007f9b0: d0 01 20 00 00 00 00 00 0b 00 00 00 00 00 00 00 
0xffff00000007f9c0: 10 00 00 00 28 00 00 00 d0 01 20 00 00 00 00 00 
0xffff00000007f9d0: 0b 00 00 00 00 00 00 00 11 00 00 00 19 00 00 00 
0xffff00000007f9e0: d0 01 20 00 00 00 00 00 0b 00 00 00 00 00 00 00 
0xffff00000007f9f0: 11 00 00 00 2a 00 00 00 d0 01 20 00 00 00 00 00 
0xffff00000007fa00: 0b 00 00 00 00 00 00 00 12 00 00 00 11 00 00 00 
0xffff00000007fa10: d0 01 20 00 00 00 00 00 0b 00 00 00 00 00 00 00 
0xffff00000007fa20: 17 00 00 00 38 00 00 00 d0 01 20 00 00 00 00 00 
0xffff00000007fa30: 0b 00 00 00 00 00 00 00 1a 00 00 00 10 00 00 00 
0xffff00000007fa40: 72 65 61 64 20 69 6e 20 00 00 00 00 20 00 00 00 
0xffff00000007fa50: 4e 6f 74 41 46 69 6c 65 d0 01 20 00 00 00 00 00 
0xffff00000007fa60: 0b 00 00 00 00 00 00 00 1c 00 00 00 31 00 00 00 
0xffff00000007fa70: d0 01 20 00 00 00 00 00 0b 00 00 00 00 00 00 00 
0xffff00000007fa80: 1d 00 00 00 31 00 00 00 64 6f 6e 65 00 00 00 00 
0xffff00000007fa90: f8 02 20 00 00 00 00 00 04 00 00 00 00 00 00 00 
0xffff00000007faa0: d0 01 20 00 00 00 00 00 0b 00 00 00 00 00 00 00 
0xffff00000007fab0: 2d 00 00 00 05 00 00 00 65 78 65 63 75 74 69 6e 
0xffff00000007fac0: 67 20 73 65 6c 66 00 00 28 03 20 00 00 00 00 00 
0xffff00000007fad0: 0e 00 00 00 00 00 00 00 d0 01 20 00 00 00 00 00 
0xffff00000007fae0: 0b 00 00 00 00 00 00 00 24 00 00 00 15 00 00 00 
0xffff00000007faf0: 46 69 6c 65 53 69 7a 65 54 6f 6f 4c 61 72 67 65 
0xffff00000007fb00: 4e 6f 53 75 63 68 46 69 6c 65 73 79 73 74 65 6d 
0xffff00000007fb10: 2f 6d 6e 74 2f 62 69 6e 2f 74 65 73 74 61 70 70 
0xffff00000007fb20: 54 6f 6f 4d 61 6e 79 46 69 6c 65 73 4f 70 65 6e 
0xffff00000007fb30: 72 61 6e 67 65 20 65 6e 64 20 69 6e 64 65 78 20 
0xffff00000007fb40: 50 0e 21 00 00 00 00 00 08 00 00 00 00 00 00 00 
0xffff00000007fb50: 08 00 00 00 00 00 00 00 90 0f 21 00 00 00 00 00 
0xffff00000007fb60: 60 0e 21 00 00 00 00 00 50 0f 21 00 00 00 00 00 
0xffff00000007fb70: 00 05 0a 0f 14 19 1e 23 28 2d 32 37 3c 41 46 4b 
0xffff00000007fb80: 50 55 5a 5f 64 69 6e 73 78 7d 82 87 8c 91 63 61 

Opening a new file and writing some data into it
ext2: allocating inode 17
ext2: allocating block 763 in group 0
ext2: writing to block 763

Reading back the data written previously
ext2: looking for "test2", found inode 17
0xffff00000007fba0: 74 68 69 73 20 69 73 20 73 6f 6d 65 20 74 65 73 
0xffff00000007fbb0: 74 20 64 61 74 61 00 00 00 00 00 00 00 00 00 00 
0xffff00000007fbc0: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 
0xffff00000007fbd0: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 
0xffff00000007fbe0: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 
0xffff00000007fbf0: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 
0xffff00000007fc00: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 
0xffff00000007fc10: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 

Printing the contents of the root directory (ext2 mount)
reading dir . with inode 2
reading dir .. with inode 2
reading dir lost+found with inode 11
reading dir bin with inode 32769
reading dir test with inode 12
reading dir dev with inode 13
reading dir tmp with inode 14
reading dir testdir with inode 15
reading dir test2 with inode 17

Finished tests

loading the first processs (/bin/sh) from elf binary file
ext2: looking for "bin", found inode 32769
ext2: looking for "sh", found inode 32772
program segment 0: 6 4 offset: 40 v:200040 p:200040 size: e0
program segment 1: 1 4 offset: 0 v:200000 p:200000 size: 1870
program segment 2: 1 5 offset: 1870 v:211870 p:211870 size: 53c8
program segment 3: 6474e551 6 offset: 0 v:0 p:0 size: 0
ext2: looking for "dev", found inode 13
timer: initializing generic arm timer to trigger context switch
kernel initialization complete
scheduler: starting multitasking
Instruction or Data Abort caused by Access Flag at address 215a70 (allocating new page)
Instruction or Data Abort caused by Access Flag at address fffffff0 (allocating new page)
Instruction or Data Abort caused by Access Flag at address 21337c (allocating new page)
Instruction or Data Abort caused by Access Flag at address 212190 (allocating new page)

Starting shell...
Instruction or Data Abort caused by Access Flag at address 216c34 (allocating new page)
% <typing in ls>
Instruction or Data Abort caused by Access Flag at address 2140e8 (allocating new page)
executing /bin/ls
child pid is 3
clearing old process space
executing a new process
ext2: looking for "bin", found inode 32769
ext2: looking for "ls", found inode 32774
program segment 0: 6 4 offset: 40 v:200040 p:200040 size: e0
program segment 1: 1 4 offset: 0 v:200000 p:200000 size: 730
program segment 2: 1 5 offset: 730 v:210730 p:210730 size: 2cb8
program segment 3: 6474e551 6 offset: 0 v:0 p:0 size: 0
ext2: looking for "dev", found inode 13
Instruction or Data Abort caused by Access Flag at address 212220 (allocating new page)
Instruction or Data Abort caused by Access Flag at address fffffff0 (allocating new page)
ext2: looking for ".", found inode 2
Instruction or Data Abort caused by Access Flag at address 2133e4 (allocating new page)
Instruction or Data Abort caused by Access Flag at address 2110e4 (allocating new page)
.
..
lost+found
bin
test
dev
tmp
testdir
test2
Exiting process 3
% 
```

