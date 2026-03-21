use iced::Event;
use iced::window::Id;
use iced_layershell::to_layer_message;
use crate::panels::main::{search_bar, app_list};
use crate::app::state::Direction;

pub const WINDOW_WIDTH: u32 = 484;
pub const WINDOW_HEIGHT: u32 = 714;
pub const POPUP_HEIGHT: u32 = 32;
pub const POPUP_GAP: u32 = 3;

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
    WifiButtonClick(bool), // true = left click (go to panel), false = right click (toggle)
    GoToWifiPanel,
    GoBackToServices,
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
    /// Refresh current workspace from compositor
    RefreshWorkspace,
    /// Navigate wifi network list up
    WifiArrowUp,
    /// Navigate wifi network list down
    WifiArrowDown,
    /// Refresh wifi network scan
    WifiScanRefresh,
    /// Force refresh wifi network scan
    WifiForceScan,
    /// Open the connect prompt for the currently selected network.
    WifiOpenConnect,
    /// Close / cancel the connect prompt without connecting.
    WifiCloseConnect,
    /// Update the password field in the connect prompt.
    WifiPasswordInput(String),
    /// Execute the connection (with current password if secured).
    WifiDoConnect,
    /// Toggle password field visibility (eye button)
    WifiTogglePasswordVisibility,
    /// Forget (remove) the currently selected wifi network
    WifiForgetNetwork,
    /// Auto-connect to the best known saved network
    WifiAutoConnect,
    /// Disconnect from the currently selected (active) network
    WifiDisconnect,
    /// Open the edit-password prompt for a saved network
    WifiEditNetwork,
}
