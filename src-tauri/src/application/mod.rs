pub mod checklist_service;
pub mod config_service;
pub mod group_preset_service;
pub mod instance_service;
pub mod ports;
pub mod preset_service;
// Wired by the /remote routes in the next slice.
#[allow(dead_code)]
pub mod remote_access_service;
pub mod template_catalog;
pub mod upload_service;
