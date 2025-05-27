pub const PAGE_SIZE: usize = 8192;
const MAX_SLOTS: usize = 256;
const HEADER_SIZE: usize = std::mem::size_of::<PageHeader>();
const SLOT_SIZE: usize = std::mem::size_of::<Slot>();
const DATA_SIZE: usize = PAGE_SIZE - HEADER_SIZE - (SLOT_SIZE * MAX_SLOTS);
const TUPLE_HEADER_SIZE: usize = std::mem::size_of::<TupleHeader>();

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PageHeader {
    pub free_space: u16,
    pub total_free_slots: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Slot {
    pub offset: u16,
    pub length: u16,
    pub is_used: u8,
    pub _pad: [u8; 1],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TupleHeader {
    pub page_no: u64,
    pub slot_no: u16,
    pub _pad: [u8; 14],
}

#[repr(C)]
pub struct Page {
    pub serial: u64,
    pub id: u64,
    pub header: PageHeader,
    pub slots: [Slot; MAX_SLOTS],
    pub data: [u8; DATA_SIZE],
}

impl PageHeader {
    fn new() -> Self {
        PageHeader {
            free_space: DATA_SIZE as u16,
            total_free_slots: MAX_SLOTS as u16,
        }
    }
}

impl TupleHeader {
    fn new(page_no: u64, slot_no: u16) -> Self {
        TupleHeader {
            page_no,
            slot_no,
            _pad: [0u8; 14],
        }
    }

    pub fn to_bytes(&self) -> [u8; TUPLE_HEADER_SIZE] {
        let mut buf = [0u8; TUPLE_HEADER_SIZE];
        buf[..8].copy_from_slice(&self.page_no.to_le_bytes());
        buf[8..10].copy_from_slice(&self.slot_no.to_le_bytes());
        buf[10..24].copy_from_slice(&self._pad);
        buf
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < TUPLE_HEADER_SIZE {
            return None;
        }

        Some(Self {
            page_no: u64::from_le_bytes(bytes[0..8].try_into().ok()?),
            slot_no: u16::from_le_bytes(bytes[8..10].try_into().ok()?),
            _pad: bytes[10..24].try_into().ok()?,
        })
    }
}

impl Slot {
    fn new() -> Self {
        Slot {
            offset: 0,
            length: 0,
            is_used: 0,
            _pad: [0; 1],
        }
    }
}

impl Page {
    pub fn new(id: u64, serial: u64) -> Self {
        Self {
            serial,
            id,
            header: PageHeader::new(),
            slots: [Slot::new(); MAX_SLOTS],
            data: [0; DATA_SIZE],
        }
    }

    fn find_free_slot(&self) -> Option<usize> {
        self.slots.iter().position(|x| x.is_used == 0)
    }

    fn find_free_space_offset(&self, length: usize) -> Option<u16> {
        let mut used_slots: Vec<(usize, usize)> = self
            .slots
            .iter()
            .filter(|x| x.is_used != 0)
            .map(|x| (x.offset as usize, (x.offset + x.length) as usize))
            .collect();
        used_slots.sort_by_key(|u| u.0);
        if let Some((offset, _)) = used_slots.first() {
            if *offset >= length {
                return Some(0);
            }
        } else {
            return Some(0);
        }
        let offset_and_length = used_slots.windows(2).find_map(|pair| {
            let (start1, end1) = pair[0];
            let (start2, _) = pair[1];
            if start2 - (start1 + end1) >= length {
                Some(end1 as u16)
            } else {
                None
            }
        });
        if offset_and_length.is_some() {
            return offset_and_length;
        }

        let (_, last_end) = *used_slots.last().unwrap();
        if last_end + length <= self.data.len() {
            return Some(last_end as u16);
        }

        None
    }

    pub fn delete_tuple(&mut self, slot_id: usize) -> Option<()> {
        self.slots[slot_id].is_used = 0;
        Some(())
    }

    pub fn update_tuple(&mut self, slot_id: usize, data: &[(usize, &[u8])]) -> Option<()> {
        let slot = self.read_slot_mut(slot_id)?;
        for (offset, col) in data.iter() {
            slot.1[*offset..*offset + col.len()].copy_from_slice(col);
        }
        Some(())
    }

    pub fn insert_tuple(&mut self, tuple_data: &[u8]) -> Result<usize, &'static str> {
        let total_tuple_len = TUPLE_HEADER_SIZE + tuple_data.len();

        if (total_tuple_len as u16) > self.header.free_space {
            return Err("Not enough free space");
        }

        let slot_index = self.find_free_slot().ok_or("No free slots available")?;
        let offset = self
            .find_free_space_offset(total_tuple_len)
            .ok_or("No free data space available")?;

        if (offset + (total_tuple_len as u16)) > self.data.len() as u16 {
            return Err("Not enough contiguous space");
        }
        let tuple_header = TupleHeader::new(self.id, slot_index as u16);

        let offset_in_usize = offset as usize;

        // add header first
        self.data[offset_in_usize..offset_in_usize + TUPLE_HEADER_SIZE]
            .copy_from_slice(&tuple_header.to_bytes());

        // add actual tuple data
        self.data[offset_in_usize + TUPLE_HEADER_SIZE..offset_in_usize + total_tuple_len]
            .copy_from_slice(tuple_data);

        let slot = &mut self.slots[slot_index];
        slot.offset = offset;
        slot.length = total_tuple_len as u16;
        slot.is_used = 1;

        self.header.free_space -= total_tuple_len as u16;
        self.header.total_free_slots -= 1;

        Ok(slot_index)
    }

    fn read_slot_mut(&mut self, index: usize) -> Option<(TupleHeader, &mut [u8])> {
        if index >= MAX_SLOTS {
            return None;
        }

        let slot = &self.slots[index];
        if slot.is_used == 0 || slot.length == 0 {
            return None;
        }

        let offset = slot.offset as usize;
        let start = offset + TUPLE_HEADER_SIZE;
        let end = offset + slot.length as usize;
        if end > self.data.len() {
            return None;
        }
        let header = TupleHeader::from_bytes(&self.data[offset..offset + TUPLE_HEADER_SIZE])?;

        Some((header, &mut self.data[start..end]))
    }

    pub fn read_slot(&self, index: usize) -> Option<(TupleHeader, &[u8])> {
        if index >= MAX_SLOTS {
            return None;
        }

        let slot = &self.slots[index];
        if slot.is_used == 0 || slot.length == 0 {
            return None;
        }

        let offset = slot.offset as usize;
        let start = offset + TUPLE_HEADER_SIZE;
        let end = offset + slot.length as usize;
        if end > self.data.len() {
            return None;
        }
        let header = TupleHeader::from_bytes(&self.data[offset..offset + TUPLE_HEADER_SIZE])?;

        Some((header, &self.data[start..end]))
    }

    pub fn to_bytes(&self) -> [u8; PAGE_SIZE] {
        let mut buf = [0u8; PAGE_SIZE];
        let mut offset = 0;

        // Serial
        buf[offset..offset + 8].copy_from_slice(&self.serial.to_le_bytes());
        offset += 8;

        // ID
        buf[offset..offset + 8].copy_from_slice(&self.id.to_le_bytes());
        offset += 8;

        // Page Header
        buf[offset..offset + 2].copy_from_slice(&self.header.free_space.to_le_bytes());
        offset += 2;

        buf[offset..offset + 2].copy_from_slice(&self.header.total_free_slots.to_le_bytes());
        offset += 2;

        // Slots
        for slot in &self.slots {
            buf[offset..offset + 2].copy_from_slice(&slot.offset.to_le_bytes());
            offset += 2;

            buf[offset..offset + 2].copy_from_slice(&slot.length.to_le_bytes());
            offset += 2;

            buf[offset] = slot.is_used;
            offset += 1;

            buf[offset] = slot._pad[0];
            offset += 1;
        }

        // Data
        buf[offset..offset + self.data.len()].copy_from_slice(&self.data);

        buf
    }

    pub fn from_bytes(buf: &[u8]) -> Result<Self, &'static str> {
        if buf.len() != PAGE_SIZE {
            return Err("Invalid page size");
        }

        let mut offset = 0;

        let serial = u64::from_le_bytes(buf[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let id = u64::from_le_bytes(buf[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let free_space = u16::from_le_bytes(buf[offset..offset + 2].try_into().unwrap());
        offset += 2;

        let total_free_slots = u16::from_le_bytes(buf[offset..offset + 2].try_into().unwrap());
        offset += 2;

        let header = PageHeader {
            free_space,
            total_free_slots,
        };

        let mut slots = [Slot::new(); MAX_SLOTS];
        for slot in &mut slots {
            slot.offset = u16::from_le_bytes(buf[offset..offset + 2].try_into().unwrap());
            offset += 2;

            slot.length = u16::from_le_bytes(buf[offset..offset + 2].try_into().unwrap());
            offset += 2;

            slot.is_used = buf[offset];
            offset += 1;

            slot._pad = [buf[offset]];
            offset += 1;
        }

        let mut data = [0u8; DATA_SIZE];
        data.copy_from_slice(&buf[offset..offset + DATA_SIZE]);

        Ok(Page {
            serial,
            id,
            header,
            slots,
            data,
        })
    }
}
