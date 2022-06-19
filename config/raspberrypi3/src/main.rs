#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;
 
use ruxpin_kernel::notice;
use ruxpin_kernel::errors::KernelError;
use ruxpin_kernel::printk::printk_dump_slice;
use ruxpin_kernel::arch::types::PhysicalAddress;

use ruxpin_kernel::irqs;
use ruxpin_kernel::fs::vfs;
use ruxpin_kernel::tasklets;
use ruxpin_kernel::mm::kmalloc;
use ruxpin_kernel::mm::vmalloc;
use ruxpin_kernel::api::binaries;
use ruxpin_kernel::proc::scheduler;

use ruxpin_types::{OpenFlags, FileAccess, Seek, DeviceID};

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

    notice!("starting kernel...");

    kmalloc::initialize(PhysicalAddress::from(0x20_0000), PhysicalAddress::from(0x100_0000));
    vmalloc::initialize(PhysicalAddress::from(0x100_0000), PhysicalAddress::from(0x1000_0000));
    irqs::register_interrupt_controller(Box::new(GenericInterruptController::new()));

    tasklets::initialize()?;
    vfs::initialize()?;
    scheduler::initialize()?;

    // Register File Systems
    vfs::register_filesystem(DevFilesystem::new())?;
    vfs::register_filesystem(ProcFilesystem::new())?;
    vfs::register_filesystem(TmpFilesystem::new())?;
    vfs::register_filesystem(Ext2Filesystem::new())?;

    // Register Drivers
    console::register()?;
    EmmcDevice::register()?;

    // Mount Root Partition
    vfs::mount(None, "/", "ext2", Some(DeviceID(0, 2)), 0).unwrap();

    // Create Mountpoints, If They Don't Exist
    check_create_directory("/dev").unwrap();
    check_create_directory("/proc").unwrap();
    check_create_directory("/tmp").unwrap();

    vfs::mount(None, "/dev", "devfs", None, 0).unwrap();
    vfs::mount(None, "/proc", "procfs", None, 0).unwrap();
    vfs::mount(None, "/tmp", "tmpfs", None, 0).unwrap();

    startup_tests().unwrap();

    // Create the first process
    notice!("loading the first processs (/bin/sh) from elf binary file");
    binaries::load_process("/bin/sh").unwrap();

    SystemTimer::init(1);

    notice!("kernel initialization complete");

    Ok(())
}

fn startup_tests() -> Result<(), KernelError> {
    if vfs::open(None, "testdir", OpenFlags::ReadOnly, FileAccess::DefaultDir, 0).is_ok() {
        notice!("\nSkipping tests because test files already exist");
        return Ok(())
    }

    notice!("\nRunning some hardcoded tests before completing the startup");

    notice!("\nCreating a directory and a file inside of it");
    vfs::open(None, "testdir", OpenFlags::Create, FileAccess::Directory.plus(FileAccess::DefaultDir), 0).unwrap();
    let file = vfs::open(None, "testdir/file.txt", OpenFlags::Create, FileAccess::DefaultFile, 0).unwrap();
    vfs::write(file.clone(), b"This is a test").unwrap();
    vfs::seek(file.clone(), 0, Seek::FromStart).unwrap();
    let mut buffer = [0; 100];
    let n = vfs::read(file.clone(), &mut buffer).unwrap();
    notice!("Read file {}: {}", n, core::str::from_utf8(&buffer).unwrap());


    notice!("\nOpening the console device file and writing to it");
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



    notice!("\nOpening the shell binary through the vfs interface and reading some data");
    let file = vfs::open(None, "/bin/sh", OpenFlags::ReadOnly, FileAccess::DefaultFile, 0).unwrap();
    let mut data = [0; 1024];
    loop {
        let nbytes = vfs::read(file.clone(), &mut data).unwrap();
        notice!("read in {} bytes", nbytes);
        printk_dump_slice(&data);
        //if nbytes != 1024 {
            break;
        //}
    }
    //vfs::close(file)?;



    notice!("\nOpening a new file and writing some data into it");
    let file = vfs::open(None, "/test2", OpenFlags::ReadWrite.plus(OpenFlags::Create), FileAccess::DefaultFile, 0).unwrap();
    vfs::write(file.clone(), b"this is some test data").unwrap();
    //vfs::close(file)?;

    notice!("\nReading back the data written previously");
    let file = vfs::open(None, "/test2", OpenFlags::ReadWrite, FileAccess::DefaultFile, 0).unwrap();
    let mut data = [0; 128];
    vfs::read(file.clone(), &mut data).unwrap();
    printk_dump_slice(&data);
    //vfs::close(file)?;

    notice!("\nPrinting the contents of the root directory (ext2 mount)");
    let file = vfs::open(None, "/", OpenFlags::ReadWrite, FileAccess::DefaultFile, 0).unwrap();
    while let Some(dirent) = vfs::readdir(file.clone()).unwrap() {
        notice!("reading dir {} with inode {}", dirent.as_str(), dirent.inode);
    }

    /*
    notice!("\nOpening a new file and writing a whole bunch of data into it");
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

    notice!("\nFinished tests\n");

    Ok(())
}

fn check_create_directory(path: &str) -> Result<(), KernelError> {
    if let Err(KernelError::FileNotFound) = vfs::open(None, path, OpenFlags::ReadOnly, FileAccess::DefaultDir, 0) {
        vfs::open(None, path, OpenFlags::Create, FileAccess::DefaultDir, 0).unwrap();
    }
    Ok(())
}
