pub mod current_window_manager;
pub mod current_window_manager_ui;

pub use current_window_manager::{get_current_workspace, get_workspaces_with_windows, switch_workspace};
pub use current_window_manager_ui::current_window_manager_view;
