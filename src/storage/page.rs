const PAGE_SIZE: usize = 8192;
const MAX_SLOTS: usize = 256;
const HEADER_SIZE: usize = std::mem::size_of::<PageHeader>();
const SLOT_SIZE: usize = std::mem::size_of::<Slot>();
const DATA_SIZE: usize = PAGE_SIZE - HEADER_SIZE - (SLOT_SIZE * MAX_SLOTS);

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
    pub xmin: u64,
    pub xmax: u64,
    pub length_of_tuple: usize,
}

#[repr(C)]
pub struct Page {
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
    fn new(xmin: u64, xmax: u64, length_of_tuple: usize) -> Self {
        TupleHeader {
            xmin,
            xmax,
            length_of_tuple,
        }
    }

    pub fn to_bytes(&self) -> [u8; 20] {
        let mut buf = [0u8; 20];
        buf[..8].copy_from_slice(&self.xmin.to_le_bytes());
        buf[8..16].copy_from_slice(&self.xmax.to_le_bytes());
        buf[16..20].copy_from_slice(&self.length_of_tuple.to_le_bytes());
        buf
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 20 {
            return None;
        }

        let xmin = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
        let xmax = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
        let length_of_tuple = usize::from_le_bytes(bytes[16..20].try_into().unwrap());

        Some(Self {
            xmin,
            xmax,
            length_of_tuple,
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
    fn new() -> Self {
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

    fn insert_tuple(
        &mut self,
        xmin: u64,
        xmax: u64,
        tuple_data: &[u8],
    ) -> Result<usize, &'static str> {
        let tuple_header_size = size_of::<TupleHeader>();
        let total_tuple_len = tuple_header_size + tuple_data.len();

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
        let tuple_header = TupleHeader::new(xmin, xmax, total_tuple_len);

        let offset_in_usize = offset as usize;

        // add header first
        self.data[offset_in_usize..offset_in_usize + tuple_header_size]
            .copy_from_slice(&tuple_header.to_bytes());

        // add actual tuple first
        self.data[offset_in_usize + tuple_header_size..offset_in_usize + total_tuple_len]
            .copy_from_slice(tuple_data);

        let slot = &mut self.slots[slot_index];
        slot.offset = offset as u16;
        slot.length = total_tuple_len as u16;
        slot.is_used = 1;

        self.header.free_space -= total_tuple_len as u16;
        self.header.total_free_slots -= 1;

        Ok(slot_index)
    }
}
