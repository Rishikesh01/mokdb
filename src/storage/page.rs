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
    pub _pad: [u8; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TupleHeader {
    pub xmin: u64,
    pub xmax: u64,
    pub page_no: u64,
    pub slot_no: u16,
    pub _pad: [u8; 6],
}

#[repr(C)]
pub struct Page {
    pub serial: u64,
    pub id: u64,
    pub header: PageHeader,
    pub _pad: [u8; 4],
    pub slots: [Slot; MAX_SLOTS],
    pub data: [u8; DATA_SIZE],
}
impl Slot {
    fn new() -> Self {
        Slot {
            offset: 0,
            length: 0,
            is_used: 0,
            _pad: [0; 3],
        }
    }
}

impl PageHeader {
    fn new() -> Self {
        PageHeader {
            free_space: DATA_SIZE as u16,
            total_free_slots: MAX_SLOTS as u16,
        }
    }
}

impl Page {
    pub fn new(serial: u64, id: u64) -> Self {
        Self {
            serial,
            id,
            header: PageHeader::new(),
            _pad: [0; 4],
            slots: [Slot::new(); MAX_SLOTS],
            data: [0; DATA_SIZE],
        }
    }
}
