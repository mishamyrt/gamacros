// pub struct AxisBus {

// }



impl AxisBus {
    pub fn new() -> Self {
        Self {
            axes: dashmap::DashMap::new(),
        }
    }
}