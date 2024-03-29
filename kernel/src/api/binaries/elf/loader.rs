
use core::mem;
use core::slice;
use alloc::string::ToString;

use ruxpin_types::{OpenFlags, FileAccess};

use crate::fs;
use crate::debug;
use crate::arch::mmu;
use crate::misc::memory;
use crate::misc::align_down;
use crate::errors::KernelError;
use crate::arch::VirtualAddress;
use crate::misc::strarray::StandardArrayOfStrings;
use crate::proc::scheduler::Task;
use crate::proc::tasks::TaskRecord;
use crate::mm::{MemoryPermissions, SegmentType};
use crate::mm::pagecache;

use super::defs::*;


pub fn load_binary(proc: Task, path: &str, argv: &StandardArrayOfStrings, envp: &StandardArrayOfStrings) -> Result<(), KernelError> {
    let mut locked_proc = proc.try_lock()?;
    locked_proc.cmd = path.to_string();

    // Open the file (if executable) and initialize the cache entry
    fs::access(locked_proc.files.try_lock()?.get_cwd(), path, FileAccess::Exec.plus(FileAccess::Regular), locked_proc.current_uid)?;
    let file = fs::open(None, path, OpenFlags::ReadOnly, FileAccess::DefaultFile, locked_proc.current_uid)?;
    let cache = pagecache::get_page_entry(file.clone())?;

    let header_page = cache.lookup_page_slice(0)?;
    let header: &Elf64Header = unsafe { memory::cast_to_ref(header_page) };

    // Look for the ELF signature, 64-bit Little Endian ELF Version 1
    if &header.e_ident[0..7] != b"\x7F\x45\x4C\x46\x02\x01\x01" {
        return Err(KernelError::NotExecutable);
    }

    // Make sure it's an executable for the Aarch64
    if header.e_type != ET_EXEC || header.e_machine != EM_AARCH64 || header.e_phentsize as usize != mem::size_of::<Elf64ProgramSegment>() {
        return Err(KernelError::NotExecutable);
    }

    const MAX_PROGRAM_SEGMENTS: usize = 12;
    if header.e_phnum as usize > MAX_PROGRAM_SEGMENTS {
        return Err(KernelError::OutOfMemory);
    }

    // Load the program headers from the ELF file
    let ps_page = cache.lookup_page_slice(header.e_phoff as usize)?;
    let program_segments: &[Elf64ProgramSegment] = unsafe {
        slice::from_raw_parts(ps_page.as_ptr().add(header.e_phoff as usize) as *const Elf64ProgramSegment, header.e_phnum as usize)
    };

    let mut end_of_data = 0;
    for (i, segment) in program_segments.iter().enumerate() {
        debug!("program segment {}: {:x} {:x} offset: {:x} v:{:x} p:{:x} size: {:x}", i, segment.p_type, segment.p_flags, segment.p_offset, segment.p_vaddr, segment.p_paddr, segment.p_filesz);

        if segment.p_type == PT_LOAD {
            let vaddr = VirtualAddress::from(segment.p_vaddr).align_down(4096);
            let offset = VirtualAddress::from(segment.p_vaddr).offset_from_align(4096);

            let permissions = flags_to_permissions(segment.p_flags)?;
            let stype = if permissions == MemoryPermissions::ReadWrite { SegmentType::Data } else { SegmentType::Text };
            locked_proc.space.try_lock()?.add_file_backed_segment(stype, permissions, cache.clone(), segment.p_offset as usize, segment.p_filesz as usize, vaddr, offset, segment.p_memsz as usize)?;
        } else if segment.p_type == PT_GNU_RELRO {
            //char **data = proc->map.segments[M_TEXT].base + prog_headers[i].p_vaddr;
            //for (int entries = prog_headers[i].p_memsz >> 2; entries; entries--, data++)
            //    *data = (char *) proc->map.segments[M_TEXT].base + (size_t) *data;
        }

        if segment.p_vaddr + segment.p_memsz > end_of_data {
            end_of_data = segment.p_vaddr + segment.p_memsz;
        }
    }
    set_up_stack(&mut *locked_proc, end_of_data, VirtualAddress::from(header.e_entry), argv, envp)?;

    Ok(())
}

fn flags_to_permissions(flags: Elf64Word) -> Result<MemoryPermissions, KernelError> {
    let rwx_flags = flags & 0x07;
    if rwx_flags == PF_R | PF_X {
        Ok(MemoryPermissions::ReadExecute)
    } else if rwx_flags == PF_R {
        Ok(MemoryPermissions::ReadOnly)
    } else if rwx_flags == PF_R | PF_W {
        Ok(MemoryPermissions::ReadWrite)
    } else {
        Err(KernelError::InvalidSegmentType)
    }
}

fn set_up_stack(locked_proc: &mut TaskRecord, end_of_data: u64, entrypoint: VirtualAddress, argv: &StandardArrayOfStrings, envp: &StandardArrayOfStrings) -> Result<(), KernelError> {
    let page_size = mmu::page_size();

    // TODO the size here is wrong, it needs to use the brk as the stack size, it needs to start higher (0x0001_0000_0000_0000 or 0x0000_8000_0000_0000)
    let stack_start = 0x1_0000_0000 as u64;
    //let stack_size = page_size * page_size;
    let stack_size = align_down((stack_start - end_of_data) as usize, page_size);

    locked_proc.space.try_lock()?.add_memory_segment(SegmentType::Stack, MemoryPermissions::ReadWrite, VirtualAddress::from(stack_start - stack_size as u64), stack_size)?;

    let argv_size = argv.calculate_size();
    let envp_size = envp.calculate_size();

    let argv_start = page_size - argv_size - envp_size;
    let envp_start = page_size - envp_size;

    let argv_base = VirtualAddress::from(stack_start - page_size as u64 + argv_start as u64);
    let envp_base = VirtualAddress::from(stack_start - page_size as u64 + envp_start as u64);

    locked_proc.space.try_lock()?.alloc_page_at(VirtualAddress::from(stack_start - page_size as u64))?;
    let page_addr = locked_proc.space.try_lock()?.translate_addr(VirtualAddress::from(stack_start - page_size as u64))?;
    let page_data: &mut [u8] = unsafe {
        core::slice::from_raw_parts_mut(page_addr.to_kernel_addr().as_mut() as *mut u8, page_size)
    };

    argv.marshall(&mut page_data[argv_start..], argv_base);
    envp.marshall(&mut page_data[envp_start..], envp_base);

    let starting_sp = VirtualAddress::from(stack_start - argv_size as u64 - envp_size as u64);

    let ttrb = locked_proc.space.try_lock()?.get_ttbr();
    locked_proc.context.init(entrypoint, starting_sp, ttrb);
    locked_proc.context.write_args(argv.offset_len(), argv_base, envp_base);

    Ok(())
}

