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
    pub xmin: u64,     // 0..8
    pub xmax: u64,     // 8..16
    pub page_no: u64,  // 16..24
    pub slot_no: u16,  // 24..26
    pub _pad: [u8; 6], // 26..32 (explicit padding)
}

#[repr(C)]
pub struct Page {
    pub header: PageHeader,
    pub slots: [Slot; MAX_SLOTS],
    pub data: [u8; DATA_SIZE],
}

trait PageManager {}

impl PageHeader {
    fn new() -> Self {
        PageHeader {
            free_space: DATA_SIZE as u16,
            total_free_slots: MAX_SLOTS as u16,
        }
    }
}

impl TupleHeader {
    fn new(xmin: u64, xmax: u64, page_no: u64, slot_no: u16) -> Self {
        TupleHeader {
            xmin,
            xmax,
            page_no,
            slot_no,
            _pad: [0u8; 6],
        }
    }
    pub fn is_visible(&self, txid: u64) -> bool {
        self.xmin <= txid && (self.xmax == 0 || txid < self.xmax)
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        let mut buf = [0u8; 32];
        buf[..8].copy_from_slice(&self.xmin.to_le_bytes());
        buf[8..16].copy_from_slice(&self.xmax.to_le_bytes());
        buf[16..24].copy_from_slice(&self.page_no.to_le_bytes());
        buf[24..26].copy_from_slice(&self.slot_no.to_le_bytes());
        buf[26..32].copy_from_slice(&self._pad); // optional, can be left zero
        buf
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 32 {
            return None;
        }

        Some(Self {
            xmin: u64::from_le_bytes(bytes[0..8].try_into().ok()?),
            xmax: u64::from_le_bytes(bytes[8..16].try_into().ok()?),
            page_no: u64::from_le_bytes(bytes[16..24].try_into().ok()?),
            slot_no: u16::from_le_bytes(bytes[24..26].try_into().ok()?),
            _pad: bytes[26..32].try_into().ok()?,
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
    pub fn new() -> Self {
        Page {
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

    pub fn insert_tuple(
        &mut self,
        xmin: u64,
        xmax: u64,
        page_no: u64,
        tuple_data: &[u8],
    ) -> Result<usize, &'static str> {
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
        let tuple_header = TupleHeader::new(xmin, xmax, page_no, slot_index as u16);

        let offset_in_usize = offset as usize;

        // add header first
        self.data[offset_in_usize..offset_in_usize + TUPLE_HEADER_SIZE]
            .copy_from_slice(&tuple_header.to_bytes());

        // add actual tuple first
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

    fn read_slot(&self, index: usize) -> Option<(TupleHeader, &[u8])> {
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

    fn scan_page(&self, txid: u64) -> Vec<(TupleHeader, &[u8])> {
        let mut result = Vec::new();

        for (i, slot) in self.slots.iter().enumerate() {
            if slot.is_used == 0 || slot.length == 0 {
                continue;
            }

            if let Some((header, data)) = self.read_slot(i) {
                if header.is_visible(txid) {
                    result.push((header, data));
                }
            }
        }

        result
    }
}
