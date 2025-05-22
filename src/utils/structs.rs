use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComponentModel {
    pub name: String,
    pub comp_type: String,
    pub comp_code: String,
    pub is_legacy: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConfigContent {
    pub repo: String,
    pub branch: String,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub comp_liste: Arc<Mutex<Vec<ComponentsList>>>,
    pub curr_ver: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComponentsList {
    pub name: String,
    pub is_legacy: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonResponse<T> {
    pub success: bool,
    pub message: Option<String>,
    pub result: Option<T>,
}

impl<T> JsonResponse<T> {
    pub fn make_error(error: String) -> Self {
        return JsonResponse {
            success: false,
            message: Some(error),
            result: None,
        };
    }
    pub fn make_success(message: &str, result: T) -> Self {
        return JsonResponse {
            success: true,
            message: Some(message.to_string()),
            result: Some(result),
        };
    }
}
