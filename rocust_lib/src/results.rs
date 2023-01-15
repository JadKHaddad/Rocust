#[derive(Clone)]
pub struct Results {
    pub succ : i32,
    pub fail : i32,
}

impl Default for Results {
    fn default() -> Self {
        Results {
            succ: 0,
            fail: 0,
        }
    }
}

impl Results {
    pub fn add_succ(&mut self, dummy: i32) {
        self.succ += 1;
    }

    pub fn add_fail(&mut self, dummy: i32) {
        self.fail += 1;
    }
}