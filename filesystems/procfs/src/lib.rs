#![no_std]

extern crate alloc;

use core::fmt::Write;

use alloc::vec;
use alloc::vec::Vec;
use alloc::sync::Arc;

use ruxpin_types::{OpenFlags, FileAccess, DeviceID, Pid, DirEntry};

use ruxpin_kernel::sync::Spinlock;
use ruxpin_kernel::errors::KernelError;
use ruxpin_kernel::misc::writer::SliceWriter;
use ruxpin_kernel::proc::scheduler;
use ruxpin_kernel::proc::tasks::TaskState;
use ruxpin_kernel::fs::generic::{GenericStaticDirectoryVnode, GenericStaticFileData};
use ruxpin_kernel::fs::types::{Filesystem, Mount, MountOperations, Vnode, VnodeOperations, FileAttributes, FilePointer};


pub struct ProcFilesystem {
    
}

struct ProcMount {
    root_node: Vnode,
    mounted_on: Option<Vnode>,
}


impl Filesystem for ProcFilesystem {
    fn fstype(&self) -> &'static str {
        "procfs"
    }

    fn init(&self) -> Result<(), KernelError> {

        Ok(())
    }

    fn mount(&mut self, parent: Option<Vnode>, _device_id: Option<DeviceID>) -> Result<Mount, KernelError> {
        let root_node = Arc::new(Spinlock::new(ProcFsRootVnode::new()));

        let mount = Arc::new(Spinlock::new(ProcMount {
            root_node,
            mounted_on: parent,
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
    attrs: FileAttributes,
}


impl ProcFsRootVnode {
    fn new() -> Self {
        Self {
            attrs: FileAttributes::new(FileAccess::DefaultDir, 0, 0),
        }
    }
}

impl VnodeOperations for ProcFsRootVnode {
    fn lookup(&mut self, filename: &str) -> Result<Vnode, KernelError> {
        let process_id = match filename.parse::<Pid>() {
            Ok(process_id) if scheduler::get_process(process_id).is_some() => Ok(process_id),
            _ => Err(KernelError::FileNotFound),
        }?;
        Ok(Arc::new(Spinlock::new(GenericStaticDirectoryVnode::new(FileAccess::DefaultReadOnlyFile, 0, 0, PROCESS_ENTRIES, process_id))))
    }

    fn attributes<'a>(&'a mut self) -> Result<&'a FileAttributes, KernelError> {
        Ok(&mut self.attrs)
    }

    //fn attributes_mut(&mut self, f: &mut dyn FnMut(&mut FileAttributes)) -> Result<(), KernelError> {
    //    Err(KernelError::OperationNotPermitted)
    //}

    fn open(&mut self, _file: &mut FilePointer, _flags: OpenFlags) -> Result<(), KernelError> {
        Ok(())
    }

    fn close(&mut self, _file: &mut FilePointer) -> Result<(), KernelError> {
        Ok(())
    }

    fn readdir(&mut self, file: &mut FilePointer) -> Result<Option<DirEntry>, KernelError> {
        if file.position >= scheduler::slot_len() {
            return Ok(None);
        }

        let proc = match scheduler::get_slot(file.position) {
            None => { return Ok(None) },
            Some(proc) => proc,
        };

        let mut result = DirEntry::new(0, b"");
        let mut writer = SliceWriter::new(&mut result.name);
        write!(writer, "{}", proc.lock().process_id).map_err(|_| KernelError::InvalidArgument)?;
        let len = writer.len();
        unsafe { result.set_len(len); }

        file.position += 1;
        Ok(Some(result))
    }
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

