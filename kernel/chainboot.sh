
CONTAINER_DIR=$(pwd)
CHAINBOOT_DIR=$(pwd)/../tools/miniload

sudo docker run -t --rm -v "$CONTAINER_DIR:/work/tutorial" -w /work/tutorial -i --privileged -v /dev:/dev -v "$CHAINBOOT_DIR:/work/common" rustembedded/osdev-utils:2021.12 ruby /work/common/minipush.rb /dev/ttyUSB0 ruxpin.img 

