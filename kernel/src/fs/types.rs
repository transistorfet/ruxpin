
use alloc::sync::Arc;

use ruxpin_api::types::{OpenFlags, FileAccess, Seek, UserID, GroupID, InodeNum, DeviceID};

use crate::sync::Spinlock;
use crate::misc::StrArray;
use crate::errors::KernelError;


pub struct DirEntry {
    pub inode: InodeNum,
    pub name: StrArray<256>,
}


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
    fn create(&mut self, _filename: &str, _access: FileAccess, _uid: UserID) -> Result<Vnode, KernelError> {
        Err(KernelError::OperationNotPermitted)
    }

    fn mknod(&mut self, _filename: &str, _access: FileAccess, _device: DeviceID, _uid: UserID) -> Result<Vnode, KernelError> {
        Err(KernelError::OperationNotPermitted)
    }

    fn lookup(&mut self, _filename: &str) -> Result<Vnode, KernelError> {
        Err(KernelError::OperationNotPermitted)
    }

    //int (*link)(struct vnode *oldvnode, struct vnode *newparent, const char *filename);
    //int (*unlink)(struct vnode *parent, struct vnode *vnode, const char *filename);
    //int (*rename)(struct vnode *vnode, struct vnode *oldparent, const char *oldname, struct vnode *newparent, const char *newname);
    //int (*truncate)(struct vnode *vnode);                        // Truncate the file data (size should be 0 after)
    //int (*update)(struct vnode *vnode);

    fn attributes<'a>(&'a mut self) -> Result<&'a FileAttributes, KernelError> {
        Err(KernelError::OperationNotPermitted)
    }
    //fn attributes_mut<'a>(&'a mut self) -> Result<&'a mut FileAttributes, KernelError>;

    fn open(&mut self, _file: &mut FilePointer, _flags: OpenFlags) -> Result<(), KernelError> {
        Err(KernelError::OperationNotPermitted)
    }

    fn close(&mut self, _file: &mut FilePointer) -> Result<(), KernelError> {
        Err(KernelError::OperationNotPermitted)
    }

    fn read(&mut self, _file: &mut FilePointer, _buffer: &mut [u8]) -> Result<usize, KernelError> {
        Err(KernelError::OperationNotPermitted)
    }

    fn write(&mut self, _file: &mut FilePointer, _buffer: &[u8]) -> Result<usize, KernelError> {
        Err(KernelError::OperationNotPermitted)
    }

    fn seek(&mut self, _file: &mut FilePointer, _offset: usize, _whence: Seek) -> Result<usize, KernelError> {
        Err(KernelError::OperationNotPermitted)
    }

    fn readdir(&mut self, _file: &mut FilePointer) -> Result<Option<DirEntry>, KernelError> {
        Err(KernelError::OperationNotPermitted)
    }

    //int (*ioctl)(struct vfile *file, unsigned int request, void *argp, uid_t uid);
    //int (*poll)(struct vfile *file, int events);
}

pub(super) struct FileAttributes {
    pub access: FileAccess,
    pub nlinks: u16,
    pub uid: UserID,
    pub gid: GroupID,
    pub rdev: Option<DeviceID>,
    pub inode: InodeNum,
    pub size: usize,

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
    pub(super) position: usize,
}

pub type File = Arc<Spinlock<FilePointer>>;


impl FilePointer {
    pub(super) fn new(vnode: Vnode) -> Self {
        Self {
            vnode,
            position: 0,
        }
    }
}

impl Default for FileAttributes {
    fn default() -> Self {
        Self {
            access: FileAccess::DefaultFile,
            nlinks: 1,
            uid: 0,
            gid: 0,
            rdev: None,
            inode: 0,
            size: 0,
        }
    }
}

