use std::sync::{Arc, Mutex};

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};

use crate::utils::{http_utils::json_response_builder, structs::JsonResponse};

use super::{
    structs::{AppState, ComponentsList, ConfigContent},
    utils::{
        execute_commande, extract_repo_info, list_dir_contents, read_from_file_ut, write_to_file_ut,
    },
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

    let (_username, repo, branch) = match extract_repo_info(&config_content) {
        Some(res) => res,
        None => {
            return json_response_builder(
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse::<String>::make_error("could not parse repository url".to_string()),
            );
        }
    };

    let files_liste = match list_dir_contents(&format!("/etc/compo-doc/tmp/{}/components", repo)) {
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

    let final_liste: Vec<ComponentsList> = files_liste
        .iter()
        .map(|file| -> ComponentsList {
            return ComponentsList {
                name: file.to_string(),
                is_legacy: false,
            };
        })
        .collect();
    return json_response_builder(
        StatusCode::INTERNAL_SERVER_ERROR,
        JsonResponse::<Vec<ComponentsList>>::make_success(
            "found components successfuly",
            final_liste,
        ),
    );
}

pub async fn get_component(Path(id): Path<u32>) -> impl IntoResponse {
    // data::find_component(id).map(Json).ok_or(AppError::NotFound)
    // return Ok(Component {});
}

pub async fn setup_config(
    State(state): State<AppState>,
    Json(config): Json<ConfigContent>,
) -> impl IntoResponse {
    // delete old config
    let _ = execute_commande("rm /etc/compo-doc/config/config");

    // create repo save foramt
    let repo_str = format!("{}/{}", &config.repo, &config.branch);
    // write the repo to file for later use
    match write_to_file_ut("/etc/compo-doc/config/config", &repo_str) {
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

    let (_, repo, branch) = match extract_repo_info(&repo_str) {
        Some(res) => res,
        None => {
            return json_response_builder(
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse::<String>::make_error("could not parse repository url".to_string()),
            );
        }
    };

    // delete old cloned repo
    let _ = execute_commande(&format!("rm -rf /etc/compo-doc/tmp/{}", &repo));

    // Execute commande to clone repo inside machine
    match execute_commande(&format!(
        "cd /etc/compo-doc/tmp && git clone -b {} --single-branch {}",
        &config.branch, &config.repo
    )) {
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

    // let fetch_version = match execute_commande(&format!("git ls-remote {} {:?}", &repo, &branch)) {
    //     Ok(v) => v.trim().split("refs").next().unwrap().to_string(),
    //     Err(err) => {
    //         print!("{}", err);
    //     }
    // };

    let mut ver = state.curr_ver;
    // Return success to user
    return json_response_builder(
        StatusCode::OK,
        JsonResponse::<String>::make_success("repository saved and reached", "OK".to_string()),
    );
}

// let fetch_version = match execute_commande(&format!(
//         "git ls-remote git@github.com:{}/{}.git {:?}",
//         &username, &folder_name, &actual_branch
//     )) {
//         Ok(v) => v.trim().split("refs").next().unwrap().to_string(),
//         Err(err) => {
//             error!("{}", err);
//             return;
//         }
//     };
