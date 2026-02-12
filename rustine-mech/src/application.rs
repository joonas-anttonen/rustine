use rustine::{gfx, gui::Key, log};

struct MyApplicationState {}

pub struct MyApplication {
    state: std::cell::RefCell<MyApplicationState>,
}

impl MyApplication {
    pub fn new() -> Self {
        MyApplication {
            state: std::cell::RefCell::new(MyApplicationState {}),
        }
    }
}

impl Drop for MyApplication {
    fn drop(&mut self) {}
}

impl rustine::gui::Application for MyApplication {
    fn startup(&self, _gui: &rustine::gui::Gui) {
        log::debug!("Application::startup");
    }

    fn on_key(&self, gui: &rustine::gui::Gui, event: rustine::gui::KeyEvent) {
        let mut _state = self.state.borrow_mut();

        if event.key == Key::UNKNOWN {
            log::warning!("Application::on_key: {:?} {:?}", event.key, event.action);
        }

        if event.action != rustine::gui::Action::PRESS {
            return;
        }

        // Assume the window is damaged after handling a key press
        gui.mark_damaged();
    }

    fn on_char(&self, gui: &rustine::gui::Gui, _c: char) {
        // Assume the window is damaged after handling a character input
        gui.mark_damaged();
    }

    fn render(&self, _gui: &rustine::gui::Gui, _frame: &mut gfx::RenderFrame) {
        let mut _state = self.state.borrow_mut();

        //_frame.fill_rectangle(rect, color);
    }
}
