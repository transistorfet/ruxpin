#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;
 
use ruxpin_kernel::printkln;
use ruxpin_kernel::printk::printk_dump;
use ruxpin_kernel::errors::KernelError;
use ruxpin_kernel::arch::types::PhysicalAddress;

use ruxpin_kernel::irqs;
use ruxpin_kernel::fs::vfs;
use ruxpin_kernel::tasklets;
use ruxpin_kernel::proc::binaries;
use ruxpin_kernel::proc::scheduler;
use ruxpin_kernel::mm::kmalloc::init_kernel_heap;
use ruxpin_kernel::mm::vmalloc::init_virtual_memory;

use ruxpin_api::types::{OpenFlags, FileAccess, Seek, DeviceID};

use ruxpin_drivers_arm::SystemTimer;
use ruxpin_drivers_arm::GenericInterruptController;
use ruxpin_drivers_raspberrypi::console;
use ruxpin_drivers_raspberrypi::emmc::EmmcDevice;

use ruxpin_filesystems_devfs::DevFilesystem;
use ruxpin_filesystems_procfs::ProcFilesystem;
use ruxpin_filesystems_tmpfs::TmpFilesystem;
use ruxpin_filesystems_ext2::Ext2Filesystem;


#[no_mangle]
pub fn register_devices() -> Result<(), KernelError> {
    console::set_safe_console();

    printkln!("starting kernel...");

    init_kernel_heap(PhysicalAddress::from(0x20_0000), PhysicalAddress::from(0x100_0000));
    init_virtual_memory(PhysicalAddress::from(0x100_0000), PhysicalAddress::from(0x1000_0000));
    irqs::register_interrupt_controller(Box::new(GenericInterruptController::new()));

    tasklets::initialize()?;
    vfs::initialize()?;
    scheduler::initialize()?;

    vfs::register_filesystem(DevFilesystem::new())?;
    vfs::register_filesystem(ProcFilesystem::new())?;
    vfs::register_filesystem(TmpFilesystem::new())?;
    vfs::register_filesystem(Ext2Filesystem::new())?;

    console::register()?;
    EmmcDevice::register()?;

    vfs::mount(None, "/", "ext2", Some(DeviceID(0, 2)), 0).unwrap();
    vfs::open(None, "/dev", OpenFlags::Create, FileAccess::DefaultDir, 0).unwrap();
    vfs::mount(None, "/dev", "devfs", None, 0).unwrap();
    vfs::open(None, "/proc", OpenFlags::Create, FileAccess::DefaultDir, 0).unwrap();
    vfs::mount(None, "/proc", "procfs", None, 0).unwrap();

    startup_tests().unwrap();

    // Create the first process
    printkln!("loading the first processs (/bin/sh) from elf binary file");
    binaries::load_process("/bin/sh").unwrap();

    SystemTimer::init(1);

    printkln!("kernel initialization complete");

    Ok(())
}

fn startup_tests() -> Result<(), KernelError> {
    printkln!("\nRunning some hardcoded tests before completing the startup");

    printkln!("\nMounting the tmpfs filesystem (simple in-memory file system)");
    vfs::open(None, "/tmp", OpenFlags::Create, FileAccess::Directory.plus(FileAccess::DefaultDir), 0).unwrap();
    vfs::mount(None, "/tmp", "tmpfs", None, 0).unwrap();

    printkln!("\nCreating a directory and a file inside of it");
    vfs::open(None, "testdir", OpenFlags::Create, FileAccess::Directory.plus(FileAccess::DefaultDir), 0).unwrap();
    let file = vfs::open(None, "testdir/file.txt", OpenFlags::Create, FileAccess::DefaultFile, 0).unwrap();
    vfs::write(file.clone(), b"This is a test").unwrap();
    vfs::seek(file.clone(), 0, Seek::FromStart).unwrap();
    let mut buffer = [0; 100];
    let n = vfs::read(file.clone(), &mut buffer).unwrap();
    printkln!("Read file {}: {}", n, core::str::from_utf8(&buffer).unwrap());


    printkln!("\nOpening the console device file and writing to it");
    let file = vfs::open(None, "/dev/console0", OpenFlags::ReadOnly, FileAccess::DefaultFile, 0).unwrap();
    vfs::write(file.clone(), b"the device file can write\n").unwrap();
    //vfs::close(file).unwrap();

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



    printkln!("\nOpening the testapp binary through the vfs interface and reading some data");
    let file = vfs::open(None, "/bin/testapp", OpenFlags::ReadOnly, FileAccess::DefaultFile, 0).unwrap();
    let mut data = [0; 1024];
    loop {
        let nbytes = vfs::read(file.clone(), &mut data).unwrap();
        printkln!("read in {} bytes", nbytes);
        unsafe { printk_dump(&data as *const u8, 1024); }
        //if nbytes != 1024 {
            break;
        //}
    }
    //vfs::close(file)?;



    printkln!("\nOpening a new file and writing some data into it");
    let file = vfs::open(None, "/test2", OpenFlags::ReadWrite.plus(OpenFlags::Create), FileAccess::DefaultFile, 0).unwrap();
    vfs::write(file.clone(), b"this is some test data").unwrap();
    //vfs::close(file)?;

    printkln!("\nReading back the data written previously");
    let file = vfs::open(None, "/test2", OpenFlags::ReadWrite, FileAccess::DefaultFile, 0).unwrap();
    let mut data = [0; 128];
    vfs::read(file.clone(), &mut data).unwrap();
    unsafe { printk_dump(&data as *const u8, 128); }
    //vfs::close(file)?;

    printkln!("\nPrinting the contents of the root directory (ext2 mount)");
    let file = vfs::open(None, "/", OpenFlags::ReadWrite, FileAccess::DefaultFile, 0).unwrap();
    while let Some(dirent) = vfs::readdir(file.clone()).unwrap() {
        printkln!("reading dir {} with inode {}", dirent.as_str(), dirent.inode);
    }

    /*
    printkln!("\nOpening a new file and writing a whole bunch of data into it");
    let file = vfs::open(None, "/test3", OpenFlags::ReadWrite.plus(OpenFlags::Create), FileAccess::DefaultFile, 0).unwrap();
    let data = [0; 4096];
    for _ in 0..20 {
        vfs::write(file.clone(), &data).unwrap();
    }
    //vfs::close(file)?;
    */

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

    printkln!("\nFinished tests\n");

    Ok(())
}

