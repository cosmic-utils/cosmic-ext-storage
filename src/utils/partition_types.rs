use storage_types::PartitionTypeInfo;

pub(crate) fn common_partition_filesystem_type(table_type: &str, index: usize) -> Option<String> {
    match table_type {
        "gpt" => storage_types::COMMON_GPT_TYPES
            .get(index)
            .map(|p: &PartitionTypeInfo| p.filesystem_type.clone()),
        "dos" => storage_types::COMMON_DOS_TYPES
            .get(index)
            .map(|p: &PartitionTypeInfo| p.filesystem_type.clone()),
        _ => None,
    }
}

pub(crate) fn common_partition_type_index_for(table_type: &str, id_type: Option<&str>) -> usize {
    let Some(id_type) = id_type else {
        return 0;
    };

    let list: &[PartitionTypeInfo] = match table_type {
        "gpt" => &storage_types::COMMON_GPT_TYPES,
        "dos" => &storage_types::COMMON_DOS_TYPES,
        _ => return 0,
    };

    list.iter()
        .position(|p| p.filesystem_type.eq_ignore_ascii_case(id_type))
        .unwrap_or(0)
}
