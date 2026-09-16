use super::*;

#[test]
fn classifies_by_extension_and_falls_back_to_other() {
    assert_eq!(classify_path(Path::new("/tmp/file.rs")), Category::Code);
    assert_eq!(classify_path(Path::new("/tmp/pic.JPG")), Category::Images);
    assert_eq!(
        classify_path(Path::new("/tmp/live.iso")),
        Category::Archives
    );
    assert_eq!(
        classify_path(Path::new("/tmp/disk.img")),
        Category::Archives
    );
    assert_eq!(classify_path(Path::new("/tmp/noext")), Category::Other);
    assert_eq!(classify_path(Path::new("/usr/bin/bash")), Category::System);
    assert_eq!(
        classify_path(Path::new("/var/cache/apt/archives/demo.deb")),
        Category::Packages
    );
}
