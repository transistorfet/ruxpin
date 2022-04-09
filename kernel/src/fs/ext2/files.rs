
use crate::block;
use crate::errors::KernelError;

use super::inodes::Ext2Vnode;


impl Ext2Vnode {
    pub(super) fn read_from_vnode(&mut self, buffer: &mut [u8], mut nbytes: usize, position: usize) -> Result<usize, KernelError> {
        let block_size = self.get_block_size();
        let device_id = self.get_device_id();

        let mut offset = 0;
	let mut znum = position / block_size;
	let mut zpos = position % block_size;

	while nbytes > 0 {
            let block_num = self.get_file_block_num(znum)?;
            let buf = block::get_buf(device_id, block_num)?;

            let zlen = if block_size - zpos <= nbytes { block_size - zpos } else { nbytes };
            let subslice = &mut buffer[offset..offset + zlen];
            subslice.copy_from_slice(&buf.block.lock()[zpos..zpos + zlen]);

            offset += zlen;
            nbytes -= zlen;
            znum += 1;
            zpos = 0;
	}

        Ok(offset)
    }
}
