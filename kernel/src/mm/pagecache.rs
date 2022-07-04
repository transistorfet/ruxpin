
use core::cmp::Ordering;
use alloc::sync::Arc;
use alloc::collections::BTreeMap;

use ruxpin_types::Seek;

use crate::arch::mmu;
use crate::arch::PhysicalAddress;
use crate::errors::KernelError;
use crate::fs::{self, Vnode, File};
use crate::sync::Spinlock;

use super::pages;


static PAGE_CACHE: Spinlock<Option<PageCache>> = Spinlock::new(None);

pub struct CachedFile(Vnode);

pub struct PageCache {
    files: BTreeMap<CachedFile, Arc<PageCacheEntry>>,
}

pub struct PageCacheEntry {
    file: File,
    pages: Spinlock<BTreeMap<usize, PhysicalAddress>>,
    
}


pub fn initialize() -> Result<(), KernelError> {
    *(PAGE_CACHE.try_lock()?) = Some(PageCache::new());
    Ok(())
}

pub fn get_page_entry(file: File) -> Result<Arc<PageCacheEntry>, KernelError> {
    Ok(PAGE_CACHE.try_lock()?.as_mut().unwrap().get(file))
}

impl PageCache {
    pub fn new() -> Self {
        Self {
            files: BTreeMap::new(),
        }
    }

    fn get(&mut self, file: File) -> Arc<PageCacheEntry> {
        let vnode = file.lock().vnode.clone();
        match self.files.get(&CachedFile(vnode.clone())) {
            Some(entry) => entry.clone(),
            None => {
                let entry = Arc::new(PageCacheEntry::new(file.clone()));
                self.files.insert(CachedFile(vnode.clone()), entry);
                self.files.get(&CachedFile(vnode)).unwrap().clone()
            }
        }
    }
}

impl PageCacheEntry {
    fn new(file: File) -> Self {
        Self {
            file,
            pages: Spinlock::new(BTreeMap::new()),
        }
    }

    pub fn lookup(&self, offset: usize) -> Result<PhysicalAddress, KernelError> {
        let pages = pages::get_page_pool();
        let page_offset = offset / mmu::page_size();

        let mut locked_pages = self.pages.try_lock()?;
        match locked_pages.get(&page_offset) {
            Some(page) => Ok(pages.ref_page(*page)),
            None => {
                let page = pages.alloc_page_zeroed();

                let page_buffer = mmu::get_page_slice(page);
                fs::seek(self.file.clone(), offset, Seek::FromStart)?;

                fs::read(self.file.clone(), &mut page_buffer[..mmu::page_size()])?;

                locked_pages.insert(page_offset, pages.ref_page(page));
                Ok(page)
            }
        }
    }

    pub fn lookup_page_slice(&self, offset: usize) -> Result<&[u8], KernelError> {
        let page = self.lookup(offset)?;
        Ok(mmu::get_page_slice(page))
    }
}

impl Eq for CachedFile { }

impl PartialEq for CachedFile {
    fn eq(&self, other: &CachedFile) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Ord for CachedFile {
    fn cmp(&self, other: &CachedFile) -> Ordering {
        (Arc::as_ptr(&self.0) as *const u8 as usize).cmp(&(Arc::as_ptr(&other.0) as *const u8 as usize))
    }
}

impl PartialOrd for CachedFile {
    fn partial_cmp(&self, other: &CachedFile) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}


