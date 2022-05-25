
use ruxpin_types::OpenFlags;
use crate::errors::KernelError;

use super::types::{Vnode, FilePointer};

pub fn is_directory(vnode: Vnode) -> Result<bool, KernelError> {
    Ok(vnode.try_lock().unwrap().attributes()?.access.is_dir())
}

pub fn is_directory_empty(vnode: Vnode) -> Result<bool, KernelError> {
    let mut file = FilePointer::new(vnode.clone());
    let mut locked_vnode = vnode.try_lock().unwrap();
    locked_vnode.open(&mut file, OpenFlags::ReadOnly)?;

    while let Some(dirent) = locked_vnode.readdir(&mut file)? {
        if dirent.as_str() != "." && dirent.as_str() != ".." {
            return Ok(false);
        }
    }
    return Ok(true);
}

