
use alloc::vec::Vec;
use alloc::sync::Arc;

use ruxpin_api::types::{OpenFlags, FileAccess, Seek};

use crate::sync::Spinlock;
use crate::errors::KernelError;

use super::types::{Filesystem, Mount, Vnode, VnodeOperations, File, FilePointer, DirEntry};


static FILESYSTEMS: Spinlock<Vec<Arc<Spinlock<dyn Filesystem>>>> = Spinlock::new(Vec::new());
static MOUNTPOINTS: Spinlock<Vec<Mount>> = Spinlock::new(Vec::new());
static ROOT_NODE: Spinlock<Option<Vnode>> = Spinlock::new(None);


pub fn initialize() -> Result<(), KernelError> {
    // TODO this is temporary
    use super::tmpfs::TmpFilesystem;
    FILESYSTEMS.lock().push(Arc::new(Spinlock::new(TmpFilesystem::new())));
    use super::devfs::DevFilesystem;
    FILESYSTEMS.lock().push(Arc::new(Spinlock::new(DevFilesystem::new())));

    // TODO this is a temporary test
    mount("/", "tmpfs").unwrap();
    let mut file = open("/dev", OpenFlags::Create, FileAccess::Directory.and(FileAccess::DefaultDir)).unwrap();
    close(&mut file).unwrap();
    mount("/dev", "devfs").unwrap();

    open("test", OpenFlags::Create, FileAccess::Directory.and(FileAccess::DefaultDir)).unwrap();
    let mut file = open("test/file.txt", OpenFlags::Create, FileAccess::DefaultFile).unwrap();
    write(&mut file, b"This is a test").unwrap();
    seek(&mut file, 0, Seek::FromStart).unwrap();
    let mut buffer = [0; 100];
    let n = read(&mut file, &mut buffer).unwrap();
    crate::printkln!("Read file {}: {}", n, core::str::from_utf8(&buffer).unwrap());

    Ok(())
}

pub fn mount(path: &str, fstype: &str) -> Result<(), KernelError> {
    let fs = find_filesystem(fstype)?;

    let vnode = lookup(path).ok();
    if vnode.is_none() && path != "/" {
        return Err(KernelError::OperationNotPermitted);
    }

    let mount = fs.lock().mount(vnode.clone(), None)?;

    let root = mount.lock().get_root()?;
    if let Some(vnode) = vnode.as_ref() {
        *vnode.lock().get_mount_mut()? = Some(root);
    } else {
        *ROOT_NODE.lock() = Some(root);
    }

    MOUNTPOINTS.lock().push(mount);
    Ok(())
}

pub fn open(path: &str, flags: OpenFlags, access: FileAccess) -> Result<File, KernelError> {
    let vnode = if flags.is_set(OpenFlags::Create) {
        create(path, access)?
    } else {
        lookup(path)?
    };

    let mut file = FilePointer::new(vnode.clone());
    vnode.lock().open(&mut file, flags)?;

    Ok(Arc::new(Spinlock::new(file)))
}

pub fn close(file: &mut File) -> Result<(), KernelError> {
    let mut fptr = file.lock();
    let vnode = fptr.vnode.clone();
    vnode.lock().close(&mut *fptr)?;
    Ok(())
}

pub fn read(file: &mut File, buffer: &mut [u8]) -> Result<usize, KernelError> {
    let mut fptr = file.lock();
    let vnode = fptr.vnode.clone();
    let result = vnode.lock().read(&mut *fptr, buffer)?;
    Ok(result)
}

pub fn write(file: &mut File, buffer: &[u8]) -> Result<usize, KernelError> {
    let mut fptr = file.lock();
    let vnode = fptr.vnode.clone();
    let result = vnode.lock().write(&mut *fptr, buffer)?;
    Ok(result)
}

pub fn seek(file: &mut File, offset: usize, whence: Seek) -> Result<usize, KernelError> {
    let mut fptr = file.lock();
    let vnode = fptr.vnode.clone();
    let result = vnode.lock().seek(&mut *fptr, offset, whence)?;
    Ok(result)
}

pub fn readdir(file: &mut File) -> Result<Option<DirEntry>, KernelError> {
    let mut fptr = file.lock();
    let vnode = fptr.vnode.clone();
    let result = vnode.lock().readdir(&mut *fptr)?;
    Ok(result)
}



pub(super) fn create(path: &str, access: FileAccess) -> Result<Vnode, KernelError> {
    let (dirname, filename) = get_path_component_reverse(path);
    let vnode = lookup(dirname)?;
    let newvnode = vnode.lock().create(filename, access, 0)?;
    Ok(newvnode)
}

pub(super) fn lookup(path: &str) -> Result<Vnode, KernelError> {
    let mut current = ROOT_NODE.lock().as_ref().ok_or(KernelError::FileNotFound)?.clone();

    let mut component;
    let mut remaining = path;
    loop {
        let mounted_root_node = current.lock().get_mount_mut().ok().map(|mount| if let Some(mount) = mount { Some(mount.clone()) } else { None }).flatten();
        let mut mounted_parent_node = None;
        if mounted_root_node.is_some() {
            mounted_parent_node = Some(current);
            current = mounted_root_node.unwrap();
        }

        if remaining == "" {
            return Ok(current);
        }

        // TODO verify file access

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
                return (&path[start..i], &path[i..]);
            }
        }

        i += 1;
    }
    (&path[start..], "")
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

fn find_filesystem(fstype: &str) -> Result<Arc<Spinlock<dyn Filesystem>>, KernelError> {
    for fs in FILESYSTEMS.lock().iter() {
        if fs.lock().fstype() == fstype {
            return Ok(fs.clone());
        }
    }
    Err(KernelError::NoSuchFilesystem)
}

