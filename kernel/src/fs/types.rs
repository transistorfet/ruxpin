
use alloc::sync::{Arc, Weak};

use ruxpin_types::{OpenFlags, FileAccess, Seek, UserID, GroupID, InodeNum, DeviceID, Timestamp, DirEntry};

use crate::sync::Spinlock;
use crate::errors::KernelError;


pub trait Filesystem: Sync + Send {
    fn fstype(&self) -> &'static str; 
    fn init(&self) -> Result<(), KernelError>;
    fn mount(&mut self, parent: Option<WeakVnode>, device_id: Option<DeviceID>) -> Result<Mount, KernelError>;
}

pub trait MountOperations: Sync + Send {
    fn get_root(&mut self) -> Result<Vnode, KernelError>;
    fn sync(&mut self) -> Result<(), KernelError>;
    fn unmount(&mut self) -> Result<(), KernelError>;
}

pub trait VnodeOperations: Sync + Send {
    fn set_self(&mut self, _vnode: WeakVnode) {
        /* The default is to ignore this, which is the behaviour for files, but not directories */
    }

    fn get_mounted_mut<'a>(&'a mut self) -> Result<&'a mut Option<Vnode>, KernelError> {
        Err(KernelError::OperationNotPermitted)
    }

    fn commit(&mut self) -> Result<(), KernelError> {
        Err(KernelError::OperationNotPermitted)
    }

    fn create(&mut self, _filename: &str, _access: FileAccess, _uid: UserID, _gid: GroupID) -> Result<Vnode, KernelError> {
        Err(KernelError::OperationNotPermitted)
    }

    fn mknod(&mut self, _filename: &str, _access: FileAccess, _device: DeviceID, _uid: UserID, _gid: GroupID) -> Result<Vnode, KernelError> {
        Err(KernelError::OperationNotPermitted)
    }

    fn lookup(&mut self, _filename: &str) -> Result<Vnode, KernelError> {
        Err(KernelError::OperationNotPermitted)
    }

    fn link(&mut self, _target: Vnode, _filename: &str) -> Result<(), KernelError> {
        Err(KernelError::OperationNotPermitted)
    }

    fn unlink(&mut self, _target: Vnode, _filename: &str) -> Result<(), KernelError> {
        Err(KernelError::OperationNotPermitted)
    }

    fn rename(&mut self, _old_name: &str, _new_parent: Option<Vnode>, _new_name: &str) -> Result<(), KernelError> {
        Err(KernelError::OperationNotPermitted)
    }

    fn truncate(&mut self) -> Result<(), KernelError> {
        Err(KernelError::OperationNotPermitted)
    }

    fn attributes<'a>(&'a mut self) -> Result<&'a FileAttributes, KernelError> {
        Err(KernelError::OperationNotPermitted)
    }

    fn attributes_mut(&mut self, _func: &mut dyn FnMut(&mut FileAttributes)) -> Result<(), KernelError> {
        Err(KernelError::OperationNotPermitted)
    }

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

#[derive(Debug)]
pub struct FileAttributes {
    pub access: FileAccess,
    pub nlinks: u16,
    pub uid: UserID,
    pub gid: GroupID,
    pub rdev: Option<DeviceID>,
    pub inode: InodeNum,
    pub size: usize,

    pub atime: Timestamp,
    pub mtime: Timestamp,
    pub ctime: Timestamp,
}


pub type Mount = Arc<Spinlock<dyn MountOperations>>;
pub type Vnode = Arc<Spinlock<dyn VnodeOperations>>;
pub type WeakVnode = Weak<Spinlock<dyn VnodeOperations>>;

pub struct FilePointer {
    pub vnode: Vnode,
    pub position: usize,
}

pub type File = Arc<Spinlock<FilePointer>>;


pub fn new_vnode<T>(inner: T) -> Vnode
where
    T: VnodeOperations + 'static
{
    let vnode: Vnode = Arc::new(Spinlock::new(inner));
    vnode.lock().set_self(Arc::downgrade(&vnode));
    vnode
}

impl FilePointer {
    pub(super) fn new(vnode: Vnode) -> Self {
        Self {
            vnode,
            position: 0,
        }
    }
}

impl FileAttributes {
    pub fn new(access: FileAccess, uid: UserID, gid: GroupID) -> Self {
        Self {
            access,
            nlinks: 1,
            uid,
            gid,
            rdev: None,
            inode: 0,
            size: 0,

            atime: Timestamp(0),
            ctime: Timestamp(0),
            mtime: Timestamp(0),
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

            atime: Timestamp(0),
            ctime: Timestamp(0),
            mtime: Timestamp(0),
        }
    }
}

