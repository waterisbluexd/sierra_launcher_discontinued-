use iced::Event;
use iced::window::Id;
use iced_layershell::to_layer_message;
use crate::panels::{search_bar, app_list};
use crate::app::state::Direction;

pub const WINDOW_WIDTH: u32 = 484;
pub const WINDOW_HEIGHT: u32 = 714;
pub const POPUP_HEIGHT: u32 = 100;
pub const POPUP_GAP: u32 = 2;

#[to_layer_message(multi)]
#[derive(Debug, Clone)]
pub enum Message {
    IcedEvent(Event),
    CheckColors,
    SearchBarMessage(search_bar::Message),
    AppListMessage(app_list::Message),
    CyclePanel(Direction),
    MusicPlayPause,
    MusicNext,
    MusicPrevious,
    MusicProgressChanged(f32),
    MusicRefresh,
    VolumeChanged(f32),
    BrightnessChanged(f32),
    VolumeMuteToggle,
    AirplaneModeToggle,
    BrightnessMinToggle,
    WifiToggle,
    BluetoothToggle,
    EyeCareToggle,
    ToggleControlCenter,
    PowerOffTheSystem,
    RestartTheSystem,
    SleepModeTheSystem,
    ClipboardArrowUp,
    ClipboardArrowDown,
    ClipboardSelect,
    ClipboardDelete,
    PrevWallpaper,
    NextWallpaper,
    NoOp,
    SetWallpaper(usize),
    ShowWindow,
    HideWindow,
    FocusSearchBar,
    WindowReady,
    AppLaunched,
    Close(Id),
    WindowClosed(Id),
    MouseMoved(f32, f32),
    PopupHoverEnter,
    PopupHoverExit,
    PopupTick,
    CreatePopupWindow,
    /// Switch to workspace number (1-based)
    SwitchWorkspace(usize),
}
