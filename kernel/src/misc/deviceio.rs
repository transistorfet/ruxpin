
use core::ptr;

use core::marker::PhantomData;
use crate::arch::types::KernelVirtualAddress;


pub struct DeviceRegisters<T> {
    base: KernelVirtualAddress,
    data: PhantomData<T>,
}

impl<T> DeviceRegisters<T> {
    pub const fn new(base: KernelVirtualAddress) -> Self {
        Self {
            base,
            data: PhantomData,
        }
    }

    pub unsafe fn get(&self, reg: impl Into<usize>) -> T {
        ptr::read_volatile(self.base.add(reg.into()).as_ptr::<T>())
    }

    pub unsafe fn set(&self, reg: impl Into<usize>, data: T) {
        ptr::write_volatile(self.base.add(reg.into()).as_mut::<T>(), data);
    }
}




/*
mod option0 {
    use core::ptr;
    use core::marker::PhantomData;
    use crate::arch::types::KernelVirtualAddress;

    pub struct DeviceRegisters<R: Into<usize>, T> {
        base: KernelVirtualAddress,
        registers: PhantomData<R>,
        data: PhantomData<T>,
    }

    impl<R: Into<usize>, T> DeviceRegisters<R, T> {
        pub const fn new(base: KernelVirtualAddress) -> Self {
            Self {
                base,
                registers: PhantomData,
                data: PhantomData,
            }
        }

        pub unsafe fn get(&self, reg: R) -> T {
            ptr::read_volatile(self.base.add(reg.into()).as_ptr::<T>())
        }

        pub unsafe fn set(&self, reg: R, data: T) {
            ptr::write_volatile(self.base.add(reg.into()).as_mut::<T>(), data);
        }
    }



    /// Example Registers

    #[repr(usize)]
    enum TimerRegisters {
        CONTROL = 0x00,
        COUNT_LOW = 0x04,
        COMPARE_1 = 0x10,
    }

    impl From<TimerRegisters> for usize {
        fn from(reg: TimerRegisters) -> usize {
            reg as usize
        }
    }

}


mod option1 {
    use core::ptr;
    use core::marker::PhantomData;
    use crate::arch::types::KernelVirtualAddress;

    struct ReadWriteRegister<T, const OFFSET: usize>(KernelVirtualAddress, PhantomData<T>);

    impl<T, const OFFSET: usize> ReadWriteRegister<T, OFFSET> {
        pub fn new(base: KernelVirtualAddress) -> Self {
            Self(base, PhantomData)
        } 

        pub fn get(&self) -> T {
            unsafe {
                ptr::read_volatile(self.0.add(OFFSET).as_ptr::<T>())
            }
        }

        pub fn set(&self, data: T) {
            unsafe {
                ptr::write_volatile(self.0.add(OFFSET).as_mut::<T>(), data);
            }
        }
    }


    struct TimerRegisters {
        control: ReadWriteRegister<u32, 0x10>,
    }

    impl TimerRegisters {
        pub fn new(base: KernelVirtualAddress) -> Self {
            Self {
                control: ReadWriteRegister::new(base),
            }            
        }
    }

    fn usecase() {
        let regs = TimerRegisters::new(KernelVirtualAddress::new(0x3F00_3000));

        regs.control.set(regs.control.get() | 1 << 1);
    }
}

/// This option avoids using as much memory as the other one, at the expense of passing in the base whenever accessing
mod option2 {
    use core::ptr;
    use core::marker::PhantomData;
    use crate::arch::types::KernelVirtualAddress;

    struct ReadWriteRegister<T, const OFFSET: usize>(PhantomData<T>);

    impl<T, const OFFSET: usize> ReadWriteRegister<T, OFFSET> {
        pub fn new() -> Self {
            Self(PhantomData)
        } 

        pub fn get(&self, base: KernelVirtualAddress) -> T {
            unsafe {
                ptr::read_volatile(base.add(OFFSET).as_ptr::<T>())
            }
        }

        pub fn set(&self, base: KernelVirtualAddress, data: T) {
            unsafe {
                ptr::write_volatile(base.add(OFFSET).as_mut::<T>(), data);
            }
        }
    }


    struct TimerRegisters {
        base: KernelVirtualAddress,
        control: ReadWriteRegister<u32, 0x10>,
    }

    impl TimerRegisters {
        pub fn new(base: KernelVirtualAddress) -> Self {
            Self {
                base,
                control: ReadWriteRegister::new(),
            }            
        }
    }

    fn usecase() {
        let regs = TimerRegisters::new(KernelVirtualAddress::new(0x3F00_3000));

        regs.control.set(regs.base, regs.control.get(regs.base) | 1 << 1);
    }
}

mod option3 {
    use core::ptr;
    use core::marker::PhantomData;
    use crate::arch::types::KernelVirtualAddress;

    pub struct DeviceRegisters<R: Into<usize>> {
        base: KernelVirtualAddress,
        data: PhantomData<R>
    }

    impl<R: Into<usize>> DeviceRegisters<R> {
        pub const fn new(base: KernelVirtualAddress) -> Self {
            Self {
                base,
                data: PhantomData,
            }
        }

        pub unsafe fn get(&self, reg: R) -> u32 {
            ptr::read_volatile(self.base.add(reg.into()).as_ptr::<u32>())
        }

        pub unsafe fn set(&self, reg: R, data: u32) {
            ptr::write_volatile(self.base.add(reg.into()).as_mut::<u32>(), data);
        }
    }
}


mod option4 {
    use core::ptr;
    use core::marker::PhantomData;
    use crate::arch::types::KernelVirtualAddress;

    trait Register<const OFFSET: usize, T> {
        fn addr(&self, base: KernelVirtualAddress) -> KernelVirtualAddress;
    }

    trait Readable<T> {
        fn get(&self, base: KernelVirtualAddress) -> T;
    }

    trait Writable<T> {
        fn set(&self, data: T);
    }

    /*
    struct RegisterAddress<const OFFSET: usize, T>(KernelVirtualAddress, PhantomData<T>);

    impl<const OFFSET: usize, T> RegisterAddress<OFFSET, T> {
        pub const fn new(addr: KernelVirtualAddress) -> Self {
            Self(addr, PhantomData)
        }
    }
    */

    impl<const OFFSET: usize, T, A> Register<OFFSET, T> for A {
        fn addr(&self, base: KernelVirtualAddress) -> KernelVirtualAddress {
            base
        }
    }

    impl<const OFFSET: usize, T> Readable<T> for dyn Register<OFFSET, T> {
        fn get(&self, base: KernelVirtualAddress) -> T {
            unsafe {
                ptr::read_volatile(base.add(OFFSET).as_ptr::<T>())
            }
        }
    }

    /*
    impl<R, T> Writeable<T> for R where R: WriteOnlyRegister<T, _> {
        fn set(&self, data: T) {
            ptr::write_volatile(KernelVirtualAddress::new(0x3f00_1000).add(self.0.addr).as_mut(), data);
        }
    }
    */




    struct TimerRegControlType(PhantomData<dyn Register<0x00, u32>>);

    /*
    impl Register<u32> for TimerRegControlType {
        fn addr(&self) -> KernelVirtualAddress {
            self.0.addr()
        }
    }
    */

    //static TimerRegControl: TimerRegControlType = TimerRegControlType(RegisterAddress::new(KernelVirtualAddress::new(0x3F00_3000)));

    /*
    fn test() {
        let test = TimerRegControl.get();
        crate::printkln!("Result: {}", test);
    }
    */


}
*/

