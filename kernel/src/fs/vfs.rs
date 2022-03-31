
use alloc::vec::Vec;
use alloc::sync::Arc;

use crate::types::{FileFlags, FileAccess, FileNumber, UserID};
use crate::errors::KernelError;
use crate::arch::sync::Spinlock;

use super::types::{Filesystem, Mount, VnodeOperations, Vnode, File, FilePointer};


static FILESYSTEMS: Spinlock<Vec<Arc<Spinlock<dyn Filesystem>>>> = Spinlock::new(Vec::new());
static MOUNTPOINTS: Spinlock<Vec<Mount>> = Spinlock::new(Vec::new());
static ROOT_NODE: Spinlock<Option<Vnode>> = Spinlock::new(None);

pub fn initialize() -> Result<(), KernelError> {
    // TODO this is temporary
    use super::tmpfs::TmpFilesystem;
    FILESYSTEMS.lock().push(Arc::new(Spinlock::new(TmpFilesystem::new())));

    // TODO this is a temporary test
    mount("/", "tmpfs").unwrap();
    create("test", FileAccess::Directory).unwrap();

    Ok(())
}

pub fn mount(path: &str, fstype: &str) -> Result<(), KernelError> {
    // TODO this is a hack.  Only allowing mounting to root
    if path != "/" {
        return Err(KernelError::PermissionNotAllowed);
    }

    for fs in FILESYSTEMS.lock().iter() {
        if fs.lock().fstype() == fstype {
            let mount = fs.lock().mount()?;
            if ROOT_NODE.lock().is_none() {
                let root = mount.lock().get_root()?;
                *ROOT_NODE.lock() = Some(root);
            }
            MOUNTPOINTS.lock().push(mount);
            return Ok(());
        }
    }
    Err(KernelError::OutOfMemory)
}

pub fn open(path: &str, mode: FileFlags) -> Result<File, KernelError> {
    let vnode = lookup(path)?;

    let mut file = FilePointer::new(vnode.clone());
    vnode.lock().open(&mut file, mode)?;

    Ok(Arc::new(Spinlock::new(file)))
}

pub fn close(file: File) -> Result<(), KernelError> {
    let mut fptr = file.lock();
    let vnode = fptr.vnode.clone();
    vnode.lock().close(&mut *fptr)?;
    Ok(())
}

pub fn read(file: File, buffer: &mut [u8]) -> Result<usize, KernelError> {
    let mut fptr = file.lock();
    let vnode = fptr.vnode.clone();
    let result = vnode.lock().read(&mut *fptr, buffer)?;
    Ok(result)
}

pub fn write(file: File, buffer: &[u8]) -> Result<usize, KernelError> {
    let mut fptr = file.lock();
    let vnode = fptr.vnode.clone();
    let result = vnode.lock().write(&mut *fptr, buffer)?;
    Ok(result)
}


pub(super) fn create(path: &str, mode: FileAccess) -> Result<Vnode, KernelError> {
    let (dirname, filename) = get_path_component_reverse(path);
    let vnode = lookup(dirname)?;
    let newvnode = vnode.lock().create(filename, mode, 0)?;
    Ok(newvnode)
}

pub(super) fn lookup(path: &str) -> Result<Vnode, KernelError> {
    let root = ROOT_NODE.lock();
    let mut current = root.as_ref().unwrap().clone();

    let mut component = "";
    let mut remaining = path;
    loop {
        if remaining == "" {
            return Ok(current);
        }

        (component, remaining) = get_path_component(remaining);

        let vnode = current.lock().lookup(component)?;
        current = vnode;
    }
}

fn get_path_component<'a>(path: &'a str) -> (&'a str, &'a str) {
    let mut i = 0;
    let mut start = 0;
    for ch in path.chars() {
        if ch == '/' {
            if i == 0 {
                start = 1;
            } else {
                return (&path[start..i - 1], &path[i..]);
            }
        }

        i += 1;
    }
    (&path, "")
}

fn get_path_component_reverse<'a>(path: &'a str) -> (&'a str, &'a str) {
    let mut i = path.len() - 1;
    for ch in path.chars().rev() {
        if ch == '/' {
            return (&path[..i], &path[i + 1..]);
        }

        i -= 1;
    }
    ("", &path)
}

