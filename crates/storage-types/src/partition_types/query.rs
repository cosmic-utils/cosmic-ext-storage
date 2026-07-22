use super::{COMMON_DOS_TYPES, COMMON_GPT_TYPES, PARTITION_TYPES, PartitionTypeInfo};

pub fn find_by_id(type_id: String) -> Option<PartitionTypeInfo> {
    PARTITION_TYPES.iter().find(|p| p.ty == type_id).cloned()
}

pub fn get_valid_partition_names(table_type: String) -> Vec<String> {
    match table_type.as_str() {
        "gpt" => COMMON_GPT_TYPES
            .iter()
            .map(|p| format!("{} - {}", p.name, p.ty))
            .collect(),
        "dos" => COMMON_DOS_TYPES
            .iter()
            .map(|p| format!("{} - {}", p.name, p.ty))
            .collect(),
        _ => vec![],
    }
}

pub fn get_all_partition_type_infos(table_type: &str) -> Vec<PartitionTypeInfo> {
    PARTITION_TYPES
        .iter()
        .filter(|p| p.table_type == table_type)
        .cloned()
        .collect()
}
