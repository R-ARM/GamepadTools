use gamepad_gui::ToolkitBuilder;

fn main() {
    let names_str = vec!["str names:", "first", "second"];

    let mut tk = ToolkitBuilder::new("Testing")
        .tab("whatever")
        .button("idk")
        .button("stuff")
        .tab("another tab")
        .button("i am a button")
        .tab("tab from vec<str>")
        .buttons_vec(names_str)
        .build();

    while tk.tick() {
        //println!("{:#?}", tk);
    }
}
