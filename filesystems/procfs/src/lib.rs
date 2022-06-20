#![no_std]

extern crate alloc;

use core::fmt::Write;

use alloc::vec;
use alloc::vec::Vec;
use alloc::sync::Arc;

use ruxpin_types::{OpenFlags, FileAccess, DeviceID, Pid, DirEntry};

use ruxpin_kernel::fs::vfs;
use ruxpin_kernel::write_bytes;
use ruxpin_kernel::sync::Spinlock;
use ruxpin_kernel::errors::KernelError;
use ruxpin_kernel::misc::writer::SliceWriter;
use ruxpin_kernel::proc::scheduler;
use ruxpin_kernel::proc::tasks::TaskState;
use ruxpin_kernel::fs::generic::{self, GenericStaticDirectoryVnode, GenericFileVnode, GenericStaticFileData};
use ruxpin_kernel::fs::types::{new_vnode, Filesystem, Mount, MountOperations, Vnode, WeakVnode, VnodeOperations, FileAttributes, FilePointer};


pub struct ProcFilesystem {
    
}

struct ProcMount {
    root_node: Vnode,
}


impl Filesystem for ProcFilesystem {
    fn fstype(&self) -> &'static str {
        "procfs"
    }

    fn init(&self) -> Result<(), KernelError> {

        Ok(())
    }

    fn mount(&mut self, parent: Option<WeakVnode>, _device_id: Option<DeviceID>) -> Result<Mount, KernelError> {
        let root_node = new_vnode(ProcFsRootVnode::new(parent));

        let mount = Arc::new(Spinlock::new(ProcMount {
            root_node,
        }));

        Ok(mount)
    }
}

impl ProcFilesystem {
    pub fn new() -> Arc<Spinlock<dyn Filesystem>> {
        Arc::new(Spinlock::new(Self {

        }))
    }
}

impl MountOperations for ProcMount {
    fn get_root(&mut self) -> Result<Vnode, KernelError> {
        Ok(self.root_node.clone())
    }

    fn sync(&mut self) -> Result<(), KernelError> {
        Ok(())
    }

    fn unmount(&mut self) -> Result<(), KernelError> {
        Ok(())
    }
}


struct ProcFsRootVnode {
    self_vnode: Option<WeakVnode>,
    parent_vnode: Option<WeakVnode>,
    attrs: FileAttributes,
}


impl ProcFsRootVnode {
    fn new(parent_vnode: Option<WeakVnode>) -> Self {
        Self {
            self_vnode: None,
            parent_vnode: parent_vnode,
            attrs: FileAttributes::new(FileAccess::DefaultDir, 0, 0),
        }
    }
}

impl VnodeOperations for ProcFsRootVnode {
    fn set_self(&mut self, vnode: WeakVnode) {
        self.self_vnode = Some(vnode);
    }

    fn lookup(&mut self, filename: &str) -> Result<Vnode, KernelError> {
        let weak_ref = if filename == "." {
            self.self_vnode.as_ref()
        } else if filename == ".." {
            self.parent_vnode.as_ref()
        } else {
            None
        };

        if let Some(vnode) = weak_ref {
            return vnode.upgrade().ok_or(KernelError::FileNotFound);
        }

        if let Ok(process_id) = filename.parse::<Pid>() {
            if scheduler::get_process(process_id).is_some() {
                Ok(new_vnode(GenericStaticDirectoryVnode::new(self.self_vnode.clone(), FileAccess::DefaultReadOnlyFile, 0, 0, PROCESS_ENTRIES, process_id)))
            } else {
                Err(KernelError::FileNotFound)
            }
        } else {
            let data = generic::get_data_from_file_list(ROOT_ENTRIES, &(), filename)?;
            Ok(new_vnode(GenericFileVnode::with_data(FileAccess::DefaultReadOnlyFile, 0, 0, data)))
        }
    }

    fn attributes<'a>(&'a mut self) -> Result<&'a FileAttributes, KernelError> {
        Ok(&mut self.attrs)
    }

    fn open(&mut self, _file: &mut FilePointer, _flags: OpenFlags) -> Result<(), KernelError> {
        Ok(())
    }

    fn close(&mut self, _file: &mut FilePointer) -> Result<(), KernelError> {
        Ok(())
    }

    fn readdir(&mut self, file: &mut FilePointer) -> Result<Option<DirEntry>, KernelError> {
        if file.position >= scheduler::slot_len() + ROOT_ENTRIES.len() + 2 {
            return Ok(None);
        }

        if file.position == 0 {
            file.position += 1;
            Ok(Some(DirEntry::new(0, ".".as_bytes())))
        } else if file.position == 1 {
            file.position += 1;
            Ok(Some(DirEntry::new(0, "..".as_bytes())))
        } else if file.position < scheduler::slot_len() + 2 {
            let proc = match scheduler::get_slot(file.position - 2) {
                None => { return Ok(None) },
                Some(proc) => proc,
            };

            let mut result = DirEntry::new(0, b"");
            let len = write_bytes!(result.name, "{}", proc.lock().process_id);
            unsafe { result.set_len(len); }

            file.position += 1;
            Ok(Some(result))
        } else {
            let index = file.position - scheduler::slot_len() - 2;

            file.position += 1;
            Ok(Some(DirEntry::new(0, ROOT_ENTRIES[index].0.as_bytes())))
        }
    }
}


const ROOT_ENTRIES: &'static [(&'static str, GenericStaticFileData<()>)] = &[
    ("mounts", file_data_mount),
];

fn file_data_mount(_nothing: &()) -> Result<Vec<u8>, KernelError> {
    let mut data = vec![0; 128];
    let mut writer = SliceWriter::new(data.as_mut_slice());

    vfs::for_each_mount(|mount| {
        // TODO implement mount data
        write!(writer, "a mount\n");
        Ok(())
    })?;

    let len = writer.len();
    unsafe { data.set_len(len); }
    Ok(data)
}


const PROCESS_ENTRIES: &'static [(&'static str, GenericStaticFileData<Pid>)] = &[
    ("stat", file_data_stat),
    ("statm", file_data_statm),
];

fn file_data_stat(process_id: &Pid) -> Result<Vec<u8>, KernelError> {
    let proc = scheduler::get_process(*process_id).ok_or(KernelError::FileNotFound)?;
    let locked_proc = proc.try_lock().unwrap();

    let mut data = vec![0; 128];
    let mut writer = SliceWriter::new(data.as_mut_slice());
    write!(writer,
        "{} {} {} {} {} {}",
        locked_proc.process_id,
        locked_proc.cmd,
        proc_state(locked_proc.state),
        locked_proc.parent_id,
        locked_proc.process_group_id,
        locked_proc.session_id,
    ).map_err(|_| KernelError::FileNotFound)?;
    let len = writer.len();
    unsafe { data.set_len(len); }

    Ok(data)
}

fn file_data_statm(_process_id: &Pid) -> Result<Vec<u8>, KernelError> {
    Ok(vec![])
}

fn proc_state(state: TaskState) -> char {
    match state {
        TaskState::Exited => 'Z',
        TaskState::Running => 'R',
        TaskState::Blocked => 'S',
    }
}

