pub const MEM_SIZE: usize = 1 << 14;
pub type Error = Box<dyn std::error::Error + Send + Sync>;
