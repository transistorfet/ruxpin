
use alloc::boxed::Box;
 
use crate::printkln;
use crate::errors::KernelError;
use crate::arch::types::PhysicalAddress;

use crate::fs::vfs;
use crate::proc::process::init_processes;
use crate::mm::kmalloc::init_kernel_heap;
use crate::mm::vmalloc::init_virtual_memory;
use crate::irqs::register_interrupt_controller;

use ruxpin_api::types::{OpenFlags, FileAccess, Seek, DeviceID};

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
    register_interrupt_controller(Box::new(GenericInterruptController::new()));

    vfs::initialize()?;

    use crate::fs::tmpfs::TmpFilesystem;
    vfs::register_filesystem(TmpFilesystem::new())?;
    use crate::fs::devfs::DevFilesystem;
    vfs::register_filesystem(DevFilesystem::new())?;
    use crate::fs::ext2::Ext2Filesystem;
    vfs::register_filesystem(Ext2Filesystem::new())?;

    init_processes();


    // TODO this is a temporary test
    vfs::mount(None, "/", "tmpfs", None, 0).unwrap();
    let file = vfs::open(None, "/dev", OpenFlags::Create, FileAccess::Directory.plus(FileAccess::DefaultDir), 0).unwrap();
    //vfs::close(file).unwrap();
    let file = vfs::open(None, "/mnt", OpenFlags::Create, FileAccess::Directory.plus(FileAccess::DefaultDir), 0).unwrap();
    //vfs::close(file).unwrap();
    vfs::mount(None, "/dev", "devfs", None, 0).unwrap();

    vfs::open(None, "test", OpenFlags::Create, FileAccess::Directory.plus(FileAccess::DefaultDir), 0).unwrap();
    let file = vfs::open(None, "test/file.txt", OpenFlags::Create, FileAccess::DefaultFile, 0).unwrap();
    vfs::write(file.clone(), b"This is a test").unwrap();
    vfs::seek(file.clone(), 0, Seek::FromStart).unwrap();
    let mut buffer = [0; 100];
    let n = vfs::read(file.clone(), &mut buffer).unwrap();
    crate::printkln!("Read file {}: {}", n, core::str::from_utf8(&buffer).unwrap());

    console::init()?;

    let file = vfs::open(None, "/dev/console0", OpenFlags::ReadOnly, FileAccess::DefaultFile, 0).unwrap();
    vfs::write(file.clone(), b"the device file can write\n").unwrap();
    //vfs::close(file).unwrap();



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


    /*
    let device_id = DeviceID(0, 0);
    let buf = block::get_buf(device_id, 4096).unwrap();
    unsafe {
        crate::printk::printk_dump(&*buf.block.lock().as_ptr(), 1024);
    }
    (&mut *buf.block.lock())[0..16].copy_from_slice(b"a secret message");
    block::commit_buf(device_id, 4096).unwrap();
    */


    vfs::mount(None, "/mnt", "ext2", Some(DeviceID(0, 2)), 0)?;

    let file = vfs::open(None, "/mnt/bin/testapp", OpenFlags::ReadOnly, FileAccess::DefaultFile, 0)?;
    let mut data = [0; 1024];
    loop {
        let nbytes = vfs::read(file.clone(), &mut data)?;
        printkln!("read in {} bytes", nbytes);
        unsafe { crate::printk::printk_dump(&data as *const u8, 1024); }
        //if nbytes != 1024 {
            break;
        //}
    }
    //vfs::close(file)?;

    use crate::proc::process::create_process;
    use crate::proc::binaries::elf::loader;
    let proc = create_process();
    loader::load_binary(proc.clone(), "/mnt/bin/sh").unwrap();
    proc.lock().files.open(None, "/dev/console0", OpenFlags::ReadWrite, FileAccess::DefaultFile, 0).unwrap();



    let file = vfs::open(None, "/mnt/test2", OpenFlags::ReadWrite.plus(OpenFlags::Create), FileAccess::DefaultFile, 0)?;
    vfs::write(file.clone(), b"this is some test data")?;
    //vfs::close(file)?;

    let file = vfs::open(None, "/mnt/test", OpenFlags::ReadWrite, FileAccess::DefaultFile, 0)?;
    let mut data = [0; 128];
    vfs::read(file.clone(), &mut data)?;
    unsafe { crate::printk::printk_dump(&data as *const u8, 128); }
    //vfs::close(file)?;

    let file = vfs::open(None, "/mnt", OpenFlags::ReadWrite, FileAccess::DefaultFile, 0)?;
    while let Some(dirent) = vfs::readdir(file.clone())? {
        printkln!("reading dir {} with inode {}", dirent.name.as_str(), dirent.inode);
    }

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


    SystemTimer::init(1);

    printkln!("kernel initialization complete");

    Ok(())
}

