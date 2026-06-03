use anyhow::Result;
use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};

pub fn insert_text(text: &str) -> Result<()> {
    let mut clipboard = Clipboard::new()?;
    let prev = clipboard.get_text().ok();
    clipboard.set_text(text)?;

    std::thread::sleep(std::time::Duration::from_millis(50));

    let mut enigo = Enigo::new(&Settings::default())?;
    enigo.key(Key::Meta, Direction::Press)?;
    enigo.key(Key::Unicode('v'), Direction::Click)?;
    enigo.key(Key::Meta, Direction::Release)?;

    std::thread::sleep(std::time::Duration::from_millis(100));
    if let Some(p) = prev {
        let _ = clipboard.set_text(p);
    }
    Ok(())
}
