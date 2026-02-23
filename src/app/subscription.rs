use iced::{Subscription, window, event};
use crate::app::message::Message;
use crate::ipc;

pub fn subscription() -> Subscription<Message> {
    let events = event::listen().map(Message::IcedEvent);
    let frames = window::frames().map(|_| Message::CheckColors);
    let music_refresh = window::frames().map(|_| Message::MusicRefresh);
    let popup_tick = window::frames().map(|_| Message::PopupTick);
    
    let ipc_poll = window::frames()
        .filter_map(|_| {
            if ipc::poll_show() {
                Some(Message::ShowWindow)
            } else {
                None
            }
        });

    Subscription::batch(vec![events, frames, music_refresh, ipc_poll, popup_tick])
}
