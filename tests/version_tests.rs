use rustine::gfx::Version;

#[test]
fn version_equality() {
    let a = Version::new(1, 2, 3);
    let b = Version::new(1, 2, 3);
    assert_eq!(a, b);
}

#[test]
fn version_ordering_major() {
    let low = Version::new(0, 9, 9);
    let high = Version::new(1, 0, 0);
    assert!(low < high);
    assert!(high > low);
}

#[test]
fn version_ordering_minor() {
    let low = Version::new(1, 0, 0);
    let high = Version::new(1, 1, 0);
    assert!(low < high);
}

#[test]
fn version_ordering_patch() {
    let low = Version::new(1, 1, 0);
    let high = Version::new(1, 1, 1);
    assert!(low < high);
}
