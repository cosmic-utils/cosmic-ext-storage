use super::*;

#[test]
fn test_disk_info_serialization() {
    let disk = DiskInfo {
        device: "/dev/sda".to_string(),
        id: "ata-Samsung_SSD_970_EVO_S1234567890".to_string(),
        model: "Samsung SSD 970 EVO".to_string(),
        serial: "S1234567890".to_string(),
        vendor: "Samsung".to_string(),
        revision: "1B2Q".to_string(),
        size: 1000000000000,
        connection_bus: "nvme".to_string(),
        rotation_rate: None,
        removable: false,
        ejectable: false,
        media_removable: false,
        media_available: true,
        optical: false,
        optical_blank: false,
        can_power_off: false,
        is_loop: false,
        backing_file: None,
        partition_table_type: Some("gpt".to_string()),
        gpt_usable_range: None,
    };

    let json = serde_json::to_string(&disk).unwrap();
    let deserialized: DiskInfo = serde_json::from_str(&json).unwrap();

    assert_eq!(disk, deserialized);
}

#[test]
fn test_smart_status_serialization() {
    let status = SmartStatus {
        device: "/dev/sda".to_string(),
        healthy: true,
        temperature_celsius: Some(35),
        power_on_hours: Some(1234),
        power_cycle_count: Some(567),
        test_running: false,
        test_percent_remaining: None,
    };

    let json = serde_json::to_string(&status).unwrap();
    let deserialized: SmartStatus = serde_json::from_str(&json).unwrap();

    assert_eq!(status, deserialized);
}

#[test]
fn test_smart_attribute_serialization() {
    let attr = SmartAttribute {
        id: 5,
        name: "Reallocated_Sector_Ct".to_string(),
        current: 100,
        worst: 100,
        threshold: 10,
        raw_value: 0,
        failing: false,
    };

    let json = serde_json::to_string(&attr).unwrap();
    let deserialized: SmartAttribute = serde_json::from_str(&json).unwrap();

    assert_eq!(attr, deserialized);
}
