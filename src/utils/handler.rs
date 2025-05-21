use axum::{Json, extract::Path, http::StatusCode, response::IntoResponse};

use crate::utils::{http_utils::json_response_builder, structs::JsonResponse};

use super::utils::{
    execute_commande, extract_repo_info, list_dir_contents, read_from_file_ut, write_to_file_ut,
};

// API handlers
pub async fn list_components() -> impl IntoResponse {
    // read from config path to get repo link
    let config_content = match read_from_file_ut("/etc/compo-doc/config/config") {
        Ok(res) => res,
        Err(err) => {
            println!("Error : {}", err);
            return json_response_builder(
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse::<String>::make_error(
                    "could not write the repository to file please try again later".to_string(),
                ),
            );
        }
    };

    let (username, repo) = match extract_repo_info(&config_content) {
        Some(res) => res,
        None => {
            return json_response_builder(
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse::<String>::make_error("could not parse repository url".to_string()),
            );
        }
    };

    let files_liste = match list_dir_contents(&format!("/etc/compo-doc/tmp/{}", repo)) {
        Ok(res) => res,
        Err(err) => {
            println!("Error : {}", err);
            return json_response_builder(
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse::<String>::make_error(
                    "could not read the components liste (either no repo found or no components)"
                        .to_string(),
                ),
            );
        }
    };
    return json_response_builder(
        StatusCode::INTERNAL_SERVER_ERROR,
        JsonResponse::<Vec<String>>::make_success(
            "could not write the repository to file please try again later",
            files_liste,
        ),
    );
}

pub async fn get_component(Path(id): Path<u32>) -> impl IntoResponse {
    // data::find_component(id).map(Json).ok_or(AppError::NotFound)
    // return Ok(Component {});
}
pub async fn setup_config(Json(git_repo): Json<String>) -> impl IntoResponse {
    // Write the repo to file for later use
    match write_to_file_ut("/etc/compo-doc/config/config", &git_repo) {
        // Do nothing if special if succede
        Ok(res) => res,
        Err(err) => {
            // Print error to console
            println!("{}", err);

            // Return error to user
            return json_response_builder(
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse::<String>::make_error(
                    "could not write the repository to file please try again later".to_string(),
                ),
            );
        }
    };

    // Execute commande to clone repo inside machine
    match execute_commande(&format!("cd /etc/compo-doc/tmp && git clone",)) {
        Ok(r) => {
            println!("repository clone successfully : {}", r)
        }
        Err(err) => {
            println!("Error occured while cloning repository : {}", err);

            // REturn error ro user
            return json_response_builder(
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse::<String>::make_error(
                    "Error occured while cloning repository, contact admin".to_string(),
                ),
            );
        }
    }

    // Return success to user
    return json_response_builder(
        StatusCode::OK,
        JsonResponse::<String>::make_success("repository saved and reached", "OK".to_string()),
    );
}
