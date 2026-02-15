use std::process::Command;

pub fn copy_to_clipboard(text: &str) {
    let _ = Command::new("wl-copy")
        .arg(text)
        .output();
}

pub fn copy_image_to_clipboard(image_data: &[u8]) {
    let _ = Command::new("wl-copy")
        .arg("--type")
        .arg("image/png")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(image_data);
            }
            child.wait()
        });
}
