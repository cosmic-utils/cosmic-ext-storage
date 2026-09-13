use super::*;

#[test]
fn image_asset_refs_are_capability_free() {
    assert!(ImageAssetRef::new("asset:fixture-image").is_ok());
    assert!(ImageAssetRef::new("/tmp/image.img").is_err());
    assert!(ImageAssetRef::new("asset:../../etc/passwd").is_err());
}
