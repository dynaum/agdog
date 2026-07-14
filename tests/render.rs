use agdog::app::App;
use agdog::ui;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

#[test]
fn renders_full_ui_without_panicking() {
    let mut app = App::new();
    app.tick();

    let backend = TestBackend::new(140, 44);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| ui::render(f, &app)).unwrap();

    let buf = terminal.backend().buffer().clone();
    let content: String = buf.content().iter().map(|c| c.symbol()).collect();

    // Summary title, a GPU panel, and the footer keybinds should all be present.
    assert!(content.contains("agdog"));
    assert!(content.contains("GPU"));
    assert!(content.contains("quit"));
}
