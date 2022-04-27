
use alloc::boxed::Box;
use alloc::collections::vec_deque::VecDeque;

use crate::printkln;
use crate::sync::Spinlock;
use crate::errors::KernelError;


pub struct Tasklet {
    func: Box<dyn FnOnce() -> Result<(), KernelError>>,
}

unsafe impl Send for Tasklet {}
unsafe impl Sync for Tasklet {}


static TASKLET_QUEUE: Spinlock<Option<VecDeque<Tasklet>>> = Spinlock::new(None);

pub fn initialize() -> Result<(), KernelError> {
    *TASKLET_QUEUE.lock() = Some(VecDeque::new());
    Ok(())
}

pub fn schedule_tasklet(func: Box<dyn FnOnce() -> Result<(), KernelError>>) {
    TASKLET_QUEUE.lock().as_mut().unwrap().push_back(Tasklet { func });
}

pub fn run_tasklets() {
    while let Some(task) = TASKLET_QUEUE.lock().as_mut().unwrap().pop_back() {
        match (task.func)() {
            Ok(()) => { },
            Err(err) => { printkln!("tasklets: error {:?}", err); },
        }
    }
}

