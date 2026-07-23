use crate::models::UiVolume;

pub(crate) fn collect_mounted_descendants_leaf_first(node: &UiVolume) -> Vec<String> {
    let mut out = Vec::new();

    fn visit(node: &UiVolume, out: &mut Vec<String>) {
        for child in &node.children {
            visit(child, out);
        }

        if node.volume.can_mount()
            && node.volume.is_mounted()
            && let Some(device) = &node.volume.device_path
        {
            out.push(device.clone());
        }
    }

    visit(node, &mut out);
    out
}
