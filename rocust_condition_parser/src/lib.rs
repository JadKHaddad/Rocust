use syn::Expr;

// So this little guy right here will parse a condition and produce a function out of it. obviously the condition is in rust syntax
pub fn test() {
    let code = "if x > 0 { 1 } else { 2 }";
    let expr = syn::parse_str::<Expr>(code).unwrap();
    println!("{:#?}", expr);
    match &expr {
        Expr::If(_) => {
            println!("if");
        }
        _ => {
           
        }
    }
}
