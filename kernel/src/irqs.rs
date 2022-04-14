
use alloc::boxed::Box;

use crate::sync::Spinlock;
use crate::errors::KernelError;



pub trait InterruptController: Sync + Send {
    fn enable_irq(&mut self, irq: usize);
    fn disable_irq(&mut self, irq: usize);
    fn pending_irqs(&mut self) -> &mut dyn Iterator<Item=usize>;
}

type IrqHandler = fn();

const MAX_IRQS: usize = 64;

static INTERRUPT_CONTROLLER: Spinlock<Option<Box<dyn InterruptController>>> = Spinlock::new(None);
static IRQ_HANDLERS: Spinlock<[Option<IrqHandler>; MAX_IRQS]> = Spinlock::new([None; MAX_IRQS]);

pub fn register_interrupt_controller(controller: Box<dyn InterruptController>) {
    *INTERRUPT_CONTROLLER.lock() = Some(controller);
}

pub fn register_irq(irq: usize, func: IrqHandler) -> Result<(), KernelError> {
    if irq >= MAX_IRQS || IRQ_HANDLERS.lock()[irq].is_some() {
        return Err(KernelError::InvalidIrq);
    }

    IRQ_HANDLERS.lock()[irq] = Some(func);
    Ok(())
}

pub fn enable_irq(irq: usize) {
    if let Some(ctrl) = INTERRUPT_CONTROLLER.lock().as_mut() {
        ctrl.enable_irq(irq);
    }
}

pub fn disable_irq(irq: usize) {
    if let Some(ctrl) = INTERRUPT_CONTROLLER.lock().as_mut() {
        ctrl.disable_irq(irq);
    }
}

pub(crate) fn handle_irqs() {
    let handlers = IRQ_HANDLERS.lock();

    if let Some(ctrl) = INTERRUPT_CONTROLLER.lock().as_mut() {
        let iter = ctrl.pending_irqs();
        while let Some(irq) = iter.next() {
            if let Some(handler) = handlers[irq] {
                handler();
            }
        }
    }
}

