
use alloc::vec::Vec;

use ruxpin_api::types::{OpenFlags, FileAccess, Seek, UserID, FileDesc};

use crate::errors::KernelError;

use super::vfs;
use super::types::{File, Vnode, DirEntry};

const MAX_OPEN_FILES: usize = 100;

pub struct FileDescriptors(Vec<Option<File>>);

impl FileDescriptors {
    pub fn new() -> Self {
        Self(Vec::with_capacity(10))
    }

    pub fn get(&self, file_num: FileDesc) -> Result<File, KernelError> {
        self.0.get(file_num.as_usize() as usize).map(|file| file.clone()).flatten().ok_or(KernelError::BadFileNumber)
    }

    pub fn close_all(&mut self) {
        for file in self.0.iter() {
            if let Some(file) = file {
                vfs::close(file.clone());
            }
        }
        self.0.clear();
    }

    pub fn open(&mut self, cwd: Option<Vnode>, path: &str, flags: OpenFlags, access: FileAccess, current_uid: UserID) -> Result<FileDesc, KernelError> {
        let file_num = self.find_first()?;
        let file = vfs::open(cwd, path, flags, access, current_uid)?;
        self.0[file_num.as_usize() as usize] = Some(file);
        Ok(file_num)
    }

    pub fn close(&mut self, file_num: FileDesc) -> Result<(), KernelError> {
        let file = self.get(file_num)?;
        vfs::close(file)?;
        self.0[file_num.as_usize() as usize] = None;
        Ok(())
    }

    pub fn read(&mut self, file_num: FileDesc, buffer: &mut [u8]) -> Result<usize, KernelError> {
        let file = self.get(file_num)?;
        vfs::read(file, buffer)
    }

    pub fn write(&mut self, file_num: FileDesc, buffer: &[u8]) -> Result<usize, KernelError> {
        let file = self.get(file_num)?;
        vfs::write(file, buffer)
    }

    pub fn seek(&mut self, file_num: FileDesc, offset: usize, whence: Seek) -> Result<usize, KernelError> {
        let file = self.get(file_num)?;
        vfs::seek(file, offset, whence)
    }

    pub fn readdir(&mut self, file_num: FileDesc) -> Result<Option<DirEntry>, KernelError> {
        let file = self.get(file_num)?;
        vfs::readdir(file)
    }

    fn find_first(&mut self) -> Result<FileDesc, KernelError> {
        let mut i = 0;
        while i < self.0.len() && self.0[i].is_some() {
            i += 1;
        }

        if i == self.0.len() {
            if i >= MAX_OPEN_FILES {
                return Err(KernelError::TooManyFilesOpen);
            }
            self.0.push(None);
        }

        Ok(FileDesc(i))
    }
}

