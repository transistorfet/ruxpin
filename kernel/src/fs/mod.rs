
pub mod generic;

mod vfs;
mod types;
mod filedesc;

pub use vfs::{
    initialize, register_filesystem, mount, sync_all, for_each_mount,
    link, unlink, rename, access, open,
    read, write, seek, readdir,
    make_directory, is_directory, is_directory_empty,
};
pub use types::{Filesystem, MountOperations, VnodeOperations, FileAttributes, Mount, Vnode, WeakVnode, FilePointer, File, new_vnode};
pub use filedesc::{FileDescriptors, SharableFileDescriptors};

