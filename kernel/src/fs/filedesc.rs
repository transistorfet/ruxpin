
use alloc::vec::Vec;

use ruxpin_api::types::{OpenFlags, FileAccess, Seek, FileNum, UserID};

use crate::errors::KernelError;

use super::vfs;
use super::types::{File, Vnode, DirEntry};

const MAX_OPEN_FILES: usize = 100;

pub struct FileDescriptors(Vec<Option<File>>);

impl FileDescriptors {
    pub fn new() -> Self {
        Self(Vec::with_capacity(10))
    }

    pub fn get(&self, file_num: FileNum) -> Result<File, KernelError> {
        self.0.get(file_num as usize).map(|file| file.clone()).flatten().ok_or(KernelError::BadFileNumber)
    }

    pub fn open(&mut self, cwd: Option<Vnode>, path: &str, flags: OpenFlags, access: FileAccess, current_uid: UserID) -> Result<FileNum, KernelError> {
        let file_num = self.find_first()?;
        let file = vfs::open(cwd, path, flags, access, current_uid)?;
        self.0[file_num as usize] = Some(file);
        Ok(file_num)
    }

    pub fn close(&mut self, file_num: FileNum) -> Result<(), KernelError> {
        let mut file = self.get(file_num)?;
        vfs::close(&mut file)?;
        self.0[file_num as usize] = None;
        Ok(())
    }

    pub fn read(&mut self, file_num: FileNum, buffer: &mut [u8]) -> Result<usize, KernelError> {
        let mut file = self.get(file_num)?;
        vfs::read(&mut file, buffer)
    }

    pub fn write(&mut self, file_num: FileNum, buffer: &[u8]) -> Result<usize, KernelError> {
        let mut file = self.get(file_num)?;
        vfs::write(&mut file, buffer)
    }

    pub fn seek(&mut self, file_num: FileNum, offset: usize, whence: Seek) -> Result<usize, KernelError> {
        let mut file = self.get(file_num)?;
        vfs::seek(&mut file, offset, whence)
    }

    pub fn readdir(&mut self, file_num: FileNum) -> Result<Option<DirEntry>, KernelError> {
        let mut file = self.get(file_num)?;
        vfs::readdir(&mut file)
    }

    fn find_first(&mut self) -> Result<FileNum, KernelError> {
        let mut i = 0;
        while i < self.0.len() && self.0[i].is_none() {
            i += 1;
        }

        if i == self.0.len() {
            if i >= MAX_OPEN_FILES {
                return Err(KernelError::TooManyFilesOpen);
            }
            self.0.push(None);
        }

        Ok(i as FileNum)
    }
}
