
use crate::misc::memory;

pub struct Partition {
    pub base: u32,
    pub size: u32,
    pub fstype: u8,
    pub flags: u8,
}

pub struct PartitionIter<'a> {
    table: &'a [MbrPartitionEntry],
    entry: usize,
}


#[repr(C)]
struct MbrPartitionEntry {
    status: u8,
    chs_start: [u8; 3],
    fstype: u8,
    chs_end: [u8; 3],
    lba_start: u32,
    lba_sectors: u32,
}

impl Partition {
    pub fn read_mbr_partition_table_iter<'a>(buffer: &'a [u8]) -> Option<PartitionIter<'a>> {
        if buffer[0x1FE] == 0x55 && buffer[0x1FF] == 0xAA {
            let table: &[MbrPartitionEntry] = unsafe {
                memory::cast_to_slice(&buffer[0x1BE..0x1FE])
            };

            Some(PartitionIter {
                table,
                entry: 0,
            })

            //for i in 0..4 {
            //    devices[i].base = from_le32((*table.add(i)).lba_start);
            //    devices[i].size = from_le32((*table.add(i)).lba_sectors);
            //    devices[i].fstype = (*table.add(i)).fstype;
            //    devices[i].flags = (*table.add(i)).status;
            //}
        } else {
            None
        }
    }
}

impl<'a> Iterator for PartitionIter<'a> {
    type Item = Partition;

    fn next(&mut self) -> Option<Self::Item> {
        while self.entry < 4 && self.table[self.entry].lba_sectors == 0 {
            self.entry += 1;
        }

        if self.entry >= 4 {
            None
        } else {
            let i = self.entry;
            self.entry += 1;
            Some(Partition {
                base: self.table[i].lba_start,
                size: self.table[i].lba_sectors,
                fstype: self.table[i].fstype,
                flags: self.table[i].status,
            })
        }
    }
}

