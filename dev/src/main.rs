use rocust::rocust_lib;
use rocust::rocust_macros;

#[rocust_macros::user]
#[derive(Default, Clone)]
pub struct MyUser {
    a: i32,
    b: i32,
    //pub results: rocust_lib::results::Results
    //pub tasks: Vec<rocust_lib::tasks::Task<Self>>
}

#[rocust_macros::has_task]
impl MyUser {
    #[task(priority = 1)]
    pub fn foo(&mut self) {
        self.a += 1;
        println!("{}", self.a);
    }

    #[task(priority = 3)]
    pub fn bar(&mut self) {
        self.b += 1;
        println!("{}", self.b);
    }

    pub fn print(&self) {
        println!("a: {}, b: {}", self.a, self.b);
    }
}

impl rocust_lib::traits::User for MyUser {
    fn on_start(&mut self) {
        println!("on_start");
    }

    fn on_stop(&mut self) {
        println!("on_stop");
    }
}
fn main() {
    //let mut my_user = MyUser::default().with_tasks(vec![rocust_lib::tasks::Task::new(1, MyUser::foo)]);
    //let task = my_user.tasks[0].clone();
    //task.call(&mut my_user);
    //my_user.print();

    //my_struct.tasks.push(Task { priority: 1, func: MyStruct::foo });
}
