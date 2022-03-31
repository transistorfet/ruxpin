
use alloc::sync::Arc;
use alloc::boxed::Box;

use crate::errors::KernelError;
use crate::arch::sync::Spinlock;
use crate::types::{FileFlags, FileAccess, FileNumber, UserID, GroupID};


pub(super) trait Filesystem: Sync + Send {
    fn fstype(&self) -> &'static str; 
    fn init(&self) -> Result<(), KernelError>;
    fn mount(&mut self) -> Result<Mount, KernelError>;
}

pub(super) trait MountOperations: Sync + Send {
    fn get_root(&self) -> Result<Vnode, KernelError>;
    fn sync(&mut self) -> Result<(), KernelError>;
    fn unmount(&mut self) -> Result<(), KernelError>;
}

pub(super) trait VnodeOperations: Sync + Send {
    fn create(&mut self, filename: &str, mode: FileAccess, uid: UserID) -> Result<Vnode, KernelError>;
    //fn mknod(&mut self, filename: &str, mode: FileAccess, device: DeviceNum, uid: UserID) -> Result<Vnode, KernelError>;
    fn lookup(&mut self, filename: &str) -> Result<Vnode, KernelError>;

    //int (*mknod)(struct vnode *vnode, const char *filename, mode_t mode, device_t dev, uid_t uid, struct vnode **result);
    //int (*link)(struct vnode *oldvnode, struct vnode *newparent, const char *filename);
    //int (*unlink)(struct vnode *parent, struct vnode *vnode, const char *filename);
    //int (*rename)(struct vnode *vnode, struct vnode *oldparent, const char *oldname, struct vnode *newparent, const char *newname);
    //int (*truncate)(struct vnode *vnode);                        // Truncate the file data (size should be 0 after)
    //int (*update)(struct vnode *vnode);

    fn attributes<'a>(&'a mut self) -> Result<&'a FileAttributes, KernelError>;
    //fn attributes_mut<'a>(&'a mut self) -> Result<&'a mut FileAttributes, KernelError>;

    fn open(&self, file: &mut FilePointer, flags: FileFlags) -> Result<(), KernelError>;
    fn close(&self, file: &mut FilePointer) -> Result<(), KernelError>;
    fn read(&self, file: &mut FilePointer, buffer: &mut [u8]) -> Result<usize, KernelError>;
    fn write(&self, file: &mut FilePointer, buffer: &[u8]) -> Result<usize, KernelError>;
    //int (*ioctl)(struct vfile *file, unsigned int request, void *argp, uid_t uid);
    //int (*poll)(struct vfile *file, int events);
    //offset_t (*seek)(struct vfile *file, offset_t position, int whence);
    //int (*readdir)(struct vfile *file, struct dirent *dir);
}

pub(super) struct FileAttributes {
    mode: FileAccess,
    links: u16,
    uid: UserID,
    gid: GroupID,
    size: usize,
    /*
    mode_t mode;
    short nlinks;
    uid_t uid;
    gid_t gid;
    uint16_t bits;
    device_t rdev;
    inode_t ino;
    offset_t size;

    time_t atime;
    time_t mtime;
    time_t ctime;
    */
}


pub(super) type Mount = Arc<Spinlock<dyn MountOperations>>;
pub(super) type Vnode = Arc<Spinlock<dyn VnodeOperations>>;

pub struct FilePointer {
    pub(super) vnode: Vnode,
    pub(super) offset: usize,
}

pub type File = Arc<Spinlock<FilePointer>>;


impl FilePointer {
    pub(super) fn new(vnode: Vnode) -> Self {
        Self {
            vnode,
            offset: 0,
        }
    }
}

impl Default for FileAttributes {
    fn default() -> Self {
        Self {
            mode: FileAccess::DefaultFile,
            links: 1,
            uid: 0,
            gid: 0,
            size: 0,
        }
    }
}

