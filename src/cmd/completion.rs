pub fn generate_zsh_completion() {
    let text = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/shell-completion/kubetui.zsh"
    ));
    println!("{text}");
}

pub fn generate_bash_completion() {
    let text = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/shell-completion/kubetui.bash"
    ));
    println!("{text}");
}
