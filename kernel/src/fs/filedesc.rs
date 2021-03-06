
use alloc::vec::Vec;
use alloc::sync::Arc;

use ruxpin_types::{OpenFlags, FileAccess, UserID, FileDesc};

use crate::sync::Spinlock;
use crate::errors::KernelError;

use super::vfs;
use super::types::{File, Vnode};

const MAX_OPEN_FILES: usize = 100;

pub type SharableFileDescriptors = Arc<Spinlock<FileDescriptors>>;

#[derive(Clone)]
pub struct FileDescriptors {
    cwd: Option<Vnode>,
    list: Vec<Option<File>>
}

impl FileDescriptors {
    pub fn new() -> Self {
        Self {
            cwd: None,
            list: Vec::with_capacity(10)
        }
    }

    pub fn new_sharable() -> SharableFileDescriptors {
        Arc::new(Spinlock::new(Self::new()))
    }

    pub fn duplicate_table(&self) -> SharableFileDescriptors {
        Arc::new(Spinlock::new(self.clone()))
    }

    pub fn get_cwd(&self) -> Option<Vnode> {
        self.cwd.clone()
    }

    pub fn get_file(&self, file_num: FileDesc) -> Result<File, KernelError> {
        self.list.get(file_num.as_usize() as usize).map(|file| file.clone()).flatten().ok_or(KernelError::BadFileNumber)
    }

    pub fn open(&mut self, cwd: Option<Vnode>, path: &str, flags: OpenFlags, access: FileAccess, current_uid: UserID) -> Result<FileDesc, KernelError> {
        let file_num = self.find_free_slot()?;
        let file = vfs::open(cwd, path, flags, access, current_uid)?;
        self.list[file_num.as_usize()] = Some(file);
        Ok(file_num)
    }

    pub fn set_slot(&mut self, file_num: FileDesc, file: File) -> Result<(), KernelError> {
        if file_num.as_usize() > MAX_OPEN_FILES {
            return Err(KernelError::TooManyFilesOpen);
        }

        if file_num.as_usize() >= self.list.len() {
            self.list.resize(file_num.as_usize() + 1, None);
        }

        self.list[file_num.as_usize()] = Some(file);
        Ok(())
    }

    pub fn clear_slot(&mut self, file_num: FileDesc) -> Result<(), KernelError> {
        self.list[file_num.as_usize()] = None;
        Ok(())
    }

    pub fn close_all(&mut self) {
        self.list.clear();
    }

    pub fn find_free_slot(&mut self) -> Result<FileDesc, KernelError> {
        let mut i = 0;
        while i < self.list.len() && self.list[i].is_some() {
            i += 1;
        }

        if i == self.list.len() {
            if i >= MAX_OPEN_FILES {
                return Err(KernelError::TooManyFilesOpen);
            }
            self.list.push(None);
        }

        Ok(FileDesc(i))
    }
}

