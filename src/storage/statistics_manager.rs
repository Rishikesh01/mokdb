use crate::engine::query_structure::DataType;

#[repr(C)]
struct TableStaticstics {
    pub table_name: String,
    pub colum_name: String,
    pub has_index: bool,
    pub uniqueness_frac: f32,
    pub nullness_frac: f32,
    pub mcv: MCV,
}

#[repr(C)]
struct MCV {
    pub value: DataType,
    pub distinctness: f32,
}

pub struct StatisticsManager {}
