
use alloc::vec::Vec;
use alloc::sync::Arc;

use ruxpin_types::{OpenFlags, FileAccess, Seek, UserID, DeviceID, DirEntry};

use crate::printkln;
use crate::sync::Spinlock;
use crate::errors::KernelError;

use super::utils;
use super::types::{Filesystem, Mount, Vnode, File, FilePointer, FileAttributes};


static FILESYSTEMS: Spinlock<Vec<Arc<Spinlock<dyn Filesystem>>>> = Spinlock::new(Vec::new());
static MOUNTPOINTS: Spinlock<Vec<Mount>> = Spinlock::new(Vec::new());
static ROOT_NODE: Spinlock<Option<Vnode>> = Spinlock::new(None);


pub fn initialize() -> Result<(), KernelError> {
    Ok(())
}

pub fn register_filesystem(fs: Arc<Spinlock<dyn Filesystem>>) -> Result<(), KernelError> {
    printkln!("fs: registering filesystem {}", fs.lock().fstype());
    FILESYSTEMS.lock().push(fs.clone());
    fs.lock().init()?;
    Ok(())
}

pub fn mount(cwd: Option<Vnode>, path: &str, fstype: &str, device_id: Option<DeviceID>, current_uid: UserID) -> Result<(), KernelError> {
    crate::printkln!("fs: mounting {} at {}, device {:?}", fstype, path, device_id);
    if current_uid != 0 {
        return Err(KernelError::OperationNotPermitted);
    }

    let fs = find_filesystem(fstype)?;

    let vnode = lookup(cwd, path, current_uid).ok();
    if vnode.is_none() && path != "/" {
        return Err(KernelError::OperationNotPermitted);
    }

    let mount = fs.lock().mount(vnode.clone(), device_id)?;

    if let Err(err) = _link_mount_to_vnode(mount.clone(), vnode) {
        mount.lock().unmount()?;
        return Err(err);
    }

    MOUNTPOINTS.lock().push(mount);
    Ok(())
}

fn _link_mount_to_vnode(mount: Mount, vnode: Option<Vnode>) -> Result<(), KernelError> {
    let root = mount.lock().get_root()?;
    if let Some(vnode) = vnode.as_ref() {
        *vnode.lock().get_mount_mut()? = Some(root);
    } else {
        *ROOT_NODE.lock() = Some(root);
    }
    Ok(())
}


pub fn link() {
    // TODO this implementations are missing
}

pub fn unlink(cwd: Option<Vnode>, path: &str, current_uid: UserID) -> Result<(), KernelError> {
    let (dirname, filename) = get_path_component_reverse(path);
    if filename == "." || filename == ".." {
        // TODO this should use reverse lookup to find the actual name
        return Err(KernelError::InvalidArgument);
    }

    let parent = lookup(cwd, dirname, current_uid)?;

    // Verify that parent directory is writable
    if !verify_file_access(current_uid, FileAccess::Write, parent.lock().attributes()?) {
        return Err(KernelError::AccessDenied);
    }

    // We look up the file here to check permissions
    let vnode = parent.lock().lookup(filename)?;

    // Verify that the file we're trying to delete is writable
    if !verify_file_access(current_uid, FileAccess::Write, vnode.lock().attributes()?) {
        return Err(KernelError::OperationNotPermitted);
    }

    if utils::is_directory(vnode.clone())? && !utils::is_directory_empty(vnode.clone())? {
        return Err(KernelError::DirectoryNotEmpty);
    }

    parent.lock().unlink(vnode, filename)?;
    Ok(())
}

pub fn rename(cwd: Option<Vnode>, old_path: &str, new_path: &str, current_uid: UserID) -> Result<(), KernelError> {
    let (old_parent, old_dir, old_name) = rename_get_parent(cwd.clone(), old_path, current_uid)?;
    let (new_parent, new_dir, new_name) = rename_get_parent(cwd, new_path, current_uid)?;

    // TODO verify that both locations are on the same device

    old_parent.lock().rename(old_name, new_parent, new_name)?;
    Ok(())
}

fn rename_get_parent<'a>(cwd: Option<Vnode>, path: &'a str, current_uid: UserID) -> Result<(Vnode, &'a str, &'a str), KernelError> {
    let (dirname, filename) = get_path_component_reverse(path);
    if filename == "." || filename == ".." {
        return Err(KernelError::InvalidArgument);
    }

    let parent = lookup(cwd, dirname, current_uid)?;

    // Verify that the parent directory of the old location is writable and searchable
    if !verify_file_access(current_uid, FileAccess::Write.plus(FileAccess::Exec).plus(FileAccess::Directory), parent.lock().attributes()?) {
        return Err(KernelError::OperationNotPermitted);
    }

    Ok((parent, dirname, filename))
}


pub fn access(cwd: Option<Vnode>, path: &str, access: FileAccess, current_uid: UserID) -> Result<(), KernelError> {
    let vnode = lookup(cwd, path, current_uid)?;

    if !verify_file_access(current_uid, access, vnode.lock().attributes()?) {
        return Err(KernelError::OperationNotPermitted);
    }
    Ok(())
}

pub fn open(cwd: Option<Vnode>, path: &str, flags: OpenFlags, access: FileAccess, current_uid: UserID) -> Result<File, KernelError> {
    let vnode = if flags.is_set(OpenFlags::Create) {
        create(cwd, path, access, current_uid)?
    } else {
        lookup(cwd, path, current_uid)?
    };

    if !verify_file_access(current_uid, flags.required_access(), vnode.lock().attributes()?) {
        return Err(KernelError::OperationNotPermitted);
    }

    if flags.is_set(OpenFlags::Truncate) {
        vnode.lock().truncate()?;
    }

    let mut file = FilePointer::new(vnode.clone());
    if flags.is_set(OpenFlags::Append) {
        file.position = vnode.lock().attributes()?.size;
    }

    vnode.lock().open(&mut file, flags)?;

    Ok(Arc::new(Spinlock::new(file)))
}

/*
pub fn close(file: File) -> Result<(), KernelError> {
    //let mut fptr = file.lock();
    //let vnode = fptr.vnode.clone();
    //vnode.lock().close(&mut *fptr)?;
    Ok(())
}
*/

impl Drop for FilePointer {
    fn drop(&mut self) {
        let vnode = self.vnode.clone();
        vnode.lock().close(self).unwrap();
    }
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

pub fn seek(file: File, offset: usize, whence: Seek) -> Result<usize, KernelError> {
    let mut fptr = file.lock();
    let vnode = fptr.vnode.clone();
    let result = vnode.lock().seek(&mut *fptr, offset, whence)?;
    Ok(result)
}

pub fn readdir(file: File) -> Result<Option<DirEntry>, KernelError> {
    let mut fptr = file.lock();
    let vnode = fptr.vnode.clone();
    let result = vnode.lock().readdir(&mut *fptr)?;
    Ok(result)
}



pub(super) fn create(cwd: Option<Vnode>, path: &str, access: FileAccess, current_uid: UserID) -> Result<Vnode, KernelError> {
    let (dirname, filename) = get_path_component_reverse(path);
    let vnode = lookup(cwd, dirname, current_uid)?;

    if !verify_file_access(current_uid, FileAccess::Write, vnode.lock().attributes()?) {
        return Err(KernelError::OperationNotPermitted);
    }

    // TODO need to add groupid here
    let newvnode = vnode.lock().create(filename, access, current_uid, 0)?;
    Ok(newvnode)
}

pub(super) fn lookup(cwd: Option<Vnode>, path: &str, current_uid: UserID) -> Result<Vnode, KernelError> {
    let mut current = if cwd.is_none() || &path[..1] == "/" {
        ROOT_NODE.lock().as_ref().ok_or(KernelError::FileNotFound)?.clone()
    } else {
        cwd.unwrap()
    };

    let mut component;
    let mut remaining = if path.len() > 0 && &path[..1] == "/" {
        &path[1..]
    } else {
        path
    };

    loop {
        let mounted_root_node = current.lock().get_mount_mut().ok().map(|mount| if let Some(mount) = mount { Some(mount.clone()) } else { None }).flatten();
        if mounted_root_node.is_some() {
            current = mounted_root_node.unwrap();
        }

        if remaining == "" {
            return Ok(current);
        }

        if !verify_file_access(current_uid, FileAccess::Read, current.lock().attributes()?) {
            return Err(KernelError::OperationNotPermitted);
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

fn verify_file_access(current_uid: UserID, require_access: FileAccess, file_attributes: &FileAttributes) -> bool {
    if !require_access.file_type().is_zero() && file_attributes.access.file_type() != require_access.file_type() {
        return false;
    }

    if current_uid == 0 || current_uid == file_attributes.uid {
        file_attributes.access.require_owner(require_access)
    } else {
        file_attributes.access.require_everyone(require_access)
    }
}

