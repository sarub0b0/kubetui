pub fn generate_zsh_completion() {
    let text = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/shell-completion/_kubetui.zsh"
    ));
    println!("{}", text);
}

