pub struct UserInfo {
    pub id: u64,
    pub name: String,
    pub total_tasks: u64,
}

impl UserInfo {
    pub fn new(id: u64, name: String, total_tasks: u64) -> Self {
        Self {
            id,
            name,
            total_tasks,
        }
    }
}

pub struct UserPanicInfo {
    pub id: u64,
    pub name: String,
}

impl UserPanicInfo {
    pub fn new(id: u64, name: String) -> Self {
        Self { id, name }
    }
}
