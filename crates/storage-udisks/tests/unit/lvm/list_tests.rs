use super::*;

#[test]
fn parses_pvs_mapping() {
    let out = "vg0\t/dev/dm-2\nvg1\t/dev/sda3\n";
    let v = parse_pvs_vg_names(out);
    assert_eq!(v.len(), 2);
    assert_eq!(v[0].0, "vg0");
    assert_eq!(v[0].1, "/dev/dm-2");
}

#[test]
fn parses_lvs_lines() {
    let out = "/dev/vg0/root\t10737418240\n/dev/vg0/home\t2147483648\n";
    let v = parse_lvs(out, "vg0");
    assert_eq!(v.len(), 2);
    assert_eq!(v[0].device_path, "/dev/vg0/root");
    assert_eq!(v[0].name, "root");
    assert_eq!(v[0].size, 10737418240);
    assert_eq!(v[0].vg_name, "vg0");
}
