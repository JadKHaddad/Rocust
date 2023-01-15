use rocust::rocust_lib;
use rocust::rocust_macros;


#[rocust_macros::be_user]
#[derive(rocust_macros::User, Default)]
pub struct MyUser {
    a: i32,
    b: i32,

    //pub results: rocust_lib::results::Results
    //pub tasks: Vec<rocust_lib::tasks::Task<Self>>
    
}

impl MyUser {
    //#[rocust_macros::task(priority = 1)]
    pub async fn foo(&self) {
        println!("{}", self.a);
    }
}

fn main() {
    let mut my_user = MyUser::default().with_tasks(vec![rocust_lib::tasks::Task::new(1, MyUser::foo)]);
    

    //my_struct.tasks.push(Task { priority: 1, func: MyStruct::foo });
}
