pub mod system;
pub mod services;
pub mod services_bottom_row;
pub mod system_services;
pub mod wifi_panel;

pub use system::{system_panel_view, SystemPanel};
pub use services::ServicesPanel;
pub use services_bottom_row::view_bottom_row;
pub use wifi_panel::WifiPanel;
