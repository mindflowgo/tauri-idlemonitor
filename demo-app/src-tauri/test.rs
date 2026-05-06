use objc2_foundation::{NSDistributedNotificationCenter, NSNotificationCenter};
fn main() {
    let center = NSDistributedNotificationCenter::defaultCenter();
    let super_center: objc2::rc::Retained<NSNotificationCenter> = objc2::rc::Retained::into_super(center.clone());
}
