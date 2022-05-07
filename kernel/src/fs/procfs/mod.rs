
use core::fmt::Write;

use alloc::vec;
use alloc::vec::Vec;
use alloc::sync::Arc;

use ruxpin_api::types::{OpenFlags, FileAccess, DeviceID, Pid, DirEntry};

use crate::sync::Spinlock;
use crate::errors::KernelError;
use crate::misc::writer::SliceWriter;
use crate::proc::scheduler;
use crate::proc::tasks::TaskState;

use super::generic::{GenericStaticDirectoryVnode, GenericStaticFileData};
use super::types::{Filesystem, Mount, MountOperations, Vnode, VnodeOperations, FileAttributes, FilePointer};


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
        let pid = filename.parse::<Pid>().map_err(|_| KernelError::FileNotFound)?;
        Ok(Arc::new(Spinlock::new(GenericStaticDirectoryVnode::new(FileAccess::DefaultReadOnlyFile, 0, 0, PROCESS_ENTRIES, pid))))
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
        write!(writer, "{}", proc.lock().pid).map_err(|_| KernelError::InvalidArgument)?;
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

fn file_data_stat(pid: &Pid) -> Result<Vec<u8>, KernelError> {
    let proc = scheduler::get_task(*pid).ok_or(KernelError::FileNotFound)?;
    let locked_proc = proc.try_lock().unwrap();

    let mut data = vec![0; 128];
    let mut writer = SliceWriter::new(data.as_mut_slice());
    write!(writer,
        "{} {} {} {} {} {}",
        locked_proc.pid,
        locked_proc.cmd,
        proc_state(locked_proc.state),
        locked_proc.parent,
        locked_proc.pgid,
        locked_proc.session,
    ).map_err(|_| KernelError::FileNotFound)?;
    let len = writer.len();
    unsafe { data.set_len(len); }

    Ok(data)
}

fn file_data_statm(pid: &Pid) -> Result<Vec<u8>, KernelError> {
    Ok(vec![])
}

fn proc_state(state: TaskState) -> char {
    match state {
        TaskState::Exited => 'Z',
        TaskState::Running => 'R',
        TaskState::Blocked => 'S',
    }
}

