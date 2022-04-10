#![allow(dead_code)]

pub type Elf64Half = u16;

pub type Elf64Word = u32;
pub type Elf64Sword = i32;

pub type Elf64Xword = u64;
pub type Elf64Sxword = i64;

pub type Elf64Addr = u64;
pub type Elf64Off = u64;

pub type Elf64Section = u16;


// ELF file header

pub const EI_NIDENT: usize = 16;

#[repr(C)]
#[derive(Debug)]
pub struct Elf64Header {
    pub e_ident:       [u8; EI_NIDENT], // Magic number and other info
    pub e_type:        Elf64Half,       // Object file type
    pub e_machine:     Elf64Half,       // Architecture
    pub e_version:     Elf64Word,       // Object file version
    pub e_entry:       Elf64Addr,       // Entry point virtual address
    pub e_phoff:       Elf64Off,        // Program header table file offset
    pub e_shoff:       Elf64Off,        // Section header table file offset
    pub e_flags:       Elf64Word,       // Processor-specific flags
    pub e_ehsize:      Elf64Half,       // ELF header size in bytes
    pub e_phentsize:   Elf64Half,       // Program header table entry size
    pub e_phnum:       Elf64Half,       // Program header table entry count
    pub e_shentsize:   Elf64Half,       // Section header table entry size
    pub e_shnum:       Elf64Half,       // Section header table entry count
    pub e_shstrndx:    Elf64Half,       // Section header string table index
}

// Ident Array Values

pub const ELFMAG: &str                  = "\x7FELF";
pub const SELFMAG: usize                = 4;
 
pub const EI_CLASS: usize               = 4;            // File class byte index
pub const ELFCLASSNONE: u8              = 0;            // Invalid class
pub const ELFCLASS32: u8                = 1;            // 32-bit objects
pub const ELFCLASS64: u8                = 2;            // 64-bit objects
pub const ELFCLASSNUM: u8               = 3;

pub const EI_DATA: usize                = 5;            // Data encoding byte index 
pub const ELFDATANONE: u8               = 0;            // Invalid data encoding
pub const ELFDATA2LSB: u8               = 1;            // 2's complement, little endian
pub const ELFDATA2MSB: u8               = 2;            // 2's complement, big endian
pub const ELFDATANUM: u8                = 3; 

pub const EI_VERSION: usize             = 6;            // File version byte index
                                                        // Value must be EV_CURRENT
 
pub const EI_OSABI: usize               = 7;            // OS ABI identification
pub const ELFOSABI_NONE: u8             = 0;            // UNIX System V ABI
pub const ELFOSABI_SYSV: u8             = 0;            // Alias
pub const ELFOSABI_ARM_AEABI: u8        = 64;           // ARM EABI
pub const ELFOSABI_ARM: u8              = 97;           // ARM
pub const ELFOSABI_STANDALONE: u8       = 255;          // Standalone (embedded) application
 
pub const EI_ABIVERSION: usize          = 8;            // ABI version
 
pub const EI_PAD: usize                 = 9;            // Byte index of padding bytes

// Possible Object File Type Values

pub const ET_NONE: Elf64Half            = 0;            // No file type
pub const ET_REL: Elf64Half             = 1;            // Relocatable file
pub const ET_EXEC: Elf64Half            = 2;            // Executable file
pub const ET_DYN: Elf64Half             = 3;            // Shared object file
pub const ET_CORE: Elf64Half            = 4;            // Core file
pub const ET_NUM: Elf64Half             = 5;            // Number of defined types
pub const ET_LOOS: Elf64Half            = 0xfe00;       // OS-specific range start
pub const ET_HIOS: Elf64Half            = 0xfeff;       // OS-specific range end
pub const ET_LOPROC: Elf64Half          = 0xff00;       // Processor-specific range start
pub const ET_HIPROC: Elf64Half          = 0xffff;       // Processor-specific range end

// Possible Machine Type Values

pub const EM_NONE: Elf64Half            = 0;            // No machine
pub const EM_68K: Elf64Half             = 4;            // Motorola m68k family
pub const EM_AARCH64: Elf64Half         = 183;          // ARM AARCH64

// Possible Version Type Values

pub const EV_NONE: Elf64Word            = 0;            // Invalid ELF version
pub const EV_CURRENT: Elf64Word         = 1;            // Current version
pub const EV_NUM: Elf64Word             = 2;

// Program Segment

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Elf64ProgramSegment {
    pub p_type:     Elf64Word,          // Segment type
    pub p_flags:    Elf64Word,          // Segment flags
    pub p_offset:   Elf64Off,           // Segment file offset
    pub p_vaddr:    Elf64Addr,          // Segment virtual address
    pub p_paddr:    Elf64Addr,          // Segment physical address
    pub p_filesz:   Elf64Xword,         // Segment size in file
    pub p_memsz:    Elf64Xword,         // Segment size in memory
    pub p_align:    Elf64Xword,         // Segment alignment
}

// Special value for e_phnum.  This indicates that the real number of
// program headers is too large to fit into e_phnum.  Instead the real
// value is in the field sh_info of section 0.

const PN_XNUM: u16      = 0xffff;

// Possible Program Segment Type Values

pub const PT_NULL: Elf64Word            = 0;            // Program header table entry unused
pub const PT_LOAD: Elf64Word            = 1;            // Loadable program segment
pub const PT_DYNAMIC: Elf64Word         = 2;            // Dynamic linking information
pub const PT_INTERP: Elf64Word          = 3;            // Program interpreter
pub const PT_NOTE: Elf64Word            = 4;            // Auxiliary information
pub const PT_SHLIB: Elf64Word           = 5;            // Reserved
pub const PT_PHDR: Elf64Word            = 6;            // Entry for header table itself
pub const PT_TLS: Elf64Word             = 7;            // Thread-local storage segment
pub const PT_NUM: Elf64Word             = 8;            // Number of defined types
pub const PT_LOOS: Elf64Word            = 0x60000000;   // Start of OS-specific
pub const PT_GNU_EH_FRAME: Elf64Word    = 0x6474e550;   // GCC .eh_frame_hdr segment
pub const PT_GNU_STACK: Elf64Word       = 0x6474e551;   // Indicates stack executability
pub const PT_GNU_RELRO: Elf64Word       = 0x6474e552;   // Read-only after relocation
pub const PT_LOSUNW: Elf64Word          = 0x6ffffffa;
pub const PT_SUNWBSS: Elf64Word         = 0x6ffffffa;   // Sun Specific segment
pub const PT_SUNWSTACK: Elf64Word       = 0x6ffffffb;   // Stack segment
pub const PT_HISUNW: Elf64Word          = 0x6fffffff;
pub const PT_HIOS: Elf64Word            = 0x6fffffff;   // End of OS-specific
pub const PT_LOPROC: Elf64Word          = 0x70000000;   // Start of processor-specific
pub const PT_HIPROC: Elf64Word          = 0x7fffffff;   // End of processor-specific

// Possible Segment Flag Values        

pub const PF_X: Elf64Word               = 1 << 0;       // Segment is executable
pub const PF_W: Elf64Word               = 1 << 1;       // Segment is writable
pub const PF_R: Elf64Word               = 1 << 2;       // Segment is readable
pub const PF_MASKOS: Elf64Word          = 0x0ff00000;   // OS-specific
pub const PF_MASKPROC: Elf64Word        = 0xf0000000;   // Processor-specific

