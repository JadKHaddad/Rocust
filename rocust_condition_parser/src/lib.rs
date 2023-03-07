use syn::Expr;

// So this little guy right here will parse a condition and produce a function out of it. obviously the condition is in rust syntax
pub fn test() {
    let code = "if x > 0 { 1 } else { 2 }";
    let expr = syn::parse_str::<Expr>(code).unwrap();
    println!("{:#?}", expr);
    if let Expr::If(_) = &expr {
        println!("if");
    }
}
