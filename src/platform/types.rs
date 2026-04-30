pub struct LockListener {
    pub stop: Box<dyn Fn() + Send + Sync>,
}
