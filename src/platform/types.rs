pub struct LockListener {
    #[allow(dead_code)]
    pub stop: Box<dyn Fn() + Send + Sync>,
}
