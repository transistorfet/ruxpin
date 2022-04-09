 
use crate::printkln;
use crate::errors::KernelError;
use crate::block::BlockOperations;
use crate::arch::types::PhysicalAddress;

use crate::proc::process::init_processes;
use crate::mm::kmalloc::init_kernel_heap;
use crate::mm::vmalloc::init_virtual_memory;
use crate::fs::vfs;
use crate::block;

use ruxpin_api::types::{OpenFlags, FileAccess, DeviceID};

#[path = "../drivers/arm/mod.rs"]
pub mod arm;

#[path = "../drivers/raspberrypi/mod.rs"]
pub mod raspberrypi;

use self::arm::SystemTimer;
use self::arm::GenericInterruptController;
use self::raspberrypi::console;
use self::raspberrypi::emmc::EmmcDevice;

pub fn register_devices() -> Result<(), KernelError> {
    console::set_safe_console();

    printkln!("starting kernel...");

    init_kernel_heap(PhysicalAddress::from(0x20_0000), PhysicalAddress::from(0x100_0000));
    init_virtual_memory(PhysicalAddress::from(0x100_0000), PhysicalAddress::from(0x1000_0000));

    vfs::initialize()?;
    init_processes();

    console::init()?;

    let mut file = vfs::open(None, "/dev/console0", OpenFlags::ReadOnly, FileAccess::DefaultFile, 0).unwrap();
    vfs::write(&mut file, b"the device file can write\n").unwrap();
    vfs::close(&mut file).unwrap();

    printkln!("emmc: initializing");
    EmmcDevice::register()?;

    /*
    let device_id = DeviceID(0, 0);
    block::open(device_id, OpenFlags::ReadOnly).unwrap();
    let mut data = [0; 1024];
    block::read(device_id, &mut data, 0).unwrap();
    unsafe {
        crate::printk::printk_dump(&data as *const u8, 1024);
    }
    */


    vfs::mount(None, "/mnt", "ext2", Some(DeviceID(0, 2)), 0)?;
    let vnode = vfs::open(None, "/mnt/testapp", OpenFlags::ReadOnly, FileAccess::DefaultFile, 0)?;
    let mut data = [0; 1024];
    vfs::read(&mut file, &mut data)?;
    unsafe { crate::printk::printk_dump(&data as *const u8, 1024); }

    /*
    use crate::misc::cache::Cache;
    #[derive(Debug)]
    struct SpecialNumber(pub usize);

    let mut cache: Cache<SpecialNumber> = Cache::new(2);
    let thing1: Result<_, KernelError> = cache.get(|item| item.0 == 1, || Ok(SpecialNumber(1)));
    {
        let thing2: Result<_, KernelError> = cache.get(|item| item.0 == 2, || Ok(SpecialNumber(2)));
        cache.print();
    }
    let thing3: Result<_, KernelError> = cache.get(|item| item.0 == 3, || Ok(SpecialNumber(3)));
    cache.print();
    let thing1: Result<_, KernelError> = cache.get(|item| item.0 == 1, || Ok(SpecialNumber(1)));
    cache.print();
    */


    //SystemTimer::init();
    //GenericInterruptController::init();

    Ok(())
}

