use iced::{application, widget::text};

#[derive(Debug, Default)]
struct State;

#[derive(Clone, Copy, Debug)]
enum Message {

}

fn update(_state: &mut State, _msg: Message) -> iced::Task<Message> {
    iced::Task::none()
}
fn view(_state: &State) -> iced::Element<Message> {
    text("Hello World").into()
}

fn main() {
    let app = application::<State, Message, _, _>("hello", update, view);
    app.run().unwrap();
}
