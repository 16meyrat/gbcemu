use nannou::prelude::*;

enum Key {
    Up, Down, Left,Right, A, B, Start, Select
}

pub enum Message {
    WindowClosed,
    KeyUp(Key),
    KeyDown(Key)
}

pub fn run_gui() {
    nannou::app(model).run();
} 

struct Model {
    window: WindowId,
}

fn model(app: &App) -> Model {
    // Create a new window! Store the ID so we can refer to it later.
    let window = app
        .new_window()
        .with_dimensions(512, 512)
        .with_title("YAGBEUM")
        .view(view) // The function that will be called for presenting graphics to a frame.
        .event(event) // The function that will be called when the window receives events.
        .build()
        .unwrap();
    Model { window }
}

fn event(_app: &App, _model: &mut Model, _event: WindowEvent) {
}

fn view(_app: &App, _model: &Model, frame: &Frame) {
    frame.clear(CORNFLOWERBLUE);
}