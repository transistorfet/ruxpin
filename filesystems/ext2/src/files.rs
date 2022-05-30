
use ruxpin_kernel::block;
use ruxpin_kernel::debug;
use ruxpin_kernel::errors::KernelError;

use super::inodes::Ext2Vnode;
use super::blocks::GetFileBlockOp;


impl Ext2Vnode {
    pub(super) fn read_from_vnode(&mut self, buffer: &mut [u8], mut nbytes: usize, position: usize) -> Result<usize, KernelError> {
        let block_size = self.get_block_size();
        let device_id = self.get_device_id();

        let mut offset = 0;
	let mut znum = position / block_size;
	let mut zpos = position % block_size;

	while nbytes > 0 {
            let block_num = match self.get_file_block_num(znum, GetFileBlockOp::Lookup)? {
                None => { break; },
                Some(num) => num,
            };
            let buf = block::get_buf(device_id, block_num)?;

            let zlen = if block_size - zpos <= nbytes { block_size - zpos } else { nbytes };
            let subslice = &mut buffer[offset..offset + zlen];
            subslice.copy_from_slice(&buf.lock()[zpos..zpos + zlen]);

            offset += zlen;
            nbytes -= zlen;
            znum += 1;
            zpos = 0;
	}

        Ok(offset)
    }

    pub(super) fn write_to_vnode(&mut self, buffer: &[u8], mut nbytes: usize, position: usize) -> Result<usize, KernelError> {
        let block_size = self.get_block_size();
        let device_id = self.get_device_id();

        let mut offset = 0;
	let mut znum = position / block_size;
	let mut zpos = position % block_size;

	while nbytes > 0 {
            let block_num = self.get_file_block_num(znum, GetFileBlockOp::Allocate)?.unwrap();
            let buf = block::get_buf(device_id, block_num)?;

            debug!("ext2: writing to block {}", block_num);
            let zlen = if block_size - zpos <= nbytes { block_size - zpos } else { nbytes };
            let subslice = &buffer[offset..offset + zlen];
            (&mut buf.lock_mut()[zpos..zpos + zlen]).copy_from_slice(subslice);

            offset += zlen;
            nbytes -= zlen;
            znum += 1;
            zpos = 0;
	}

        Ok(offset)
    }
}

