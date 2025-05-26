use axum::{Extension, Json, extract::Path, http::StatusCode, response::IntoResponse};

use crate::utils::{
    code_merge::{get_imported_components, merge_recurse},
    http_utils::json_response_builder,
    structs::{ComponentModel, JsonResponse},
    type_extractor::find_used_type,
};

use super::{
    structs::{ComponentsList, ConfigContent, SharedState},
    type_extractor::TypeExtractor,
    utils::{
        execute_commande, extract_repo_info, get_new_repo_ver, list_dir_contents,
        read_from_file_ut, write_to_file_ut,
    },
};

// API handlers
pub async fn list_components(state: Extension<SharedState>) -> impl IntoResponse {
    let mut state = state.write().await;

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

    let (username, repo, branch) = match extract_repo_info(&config_content) {
        Some(res) => res,
        None => {
            return json_response_builder(
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse::<String>::make_error("could not parse repository url".to_string()),
            );
        }
    };

    let holding_folder = format!("/etc/compo-doc/tmp/{}/components", repo);

    let fetched_version = match execute_commande(&format!(
        "git ls-remote https://github.com/{}/{}.git {:?}",
        &username, &repo, &branch
    )) {
        Ok(v) => v.trim().split("refs").next().unwrap().to_string(),
        Err(err) => {
            print!("{}", err);
            return json_response_builder(
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse::<String>::make_error("Error while checking repo version".to_string()),
            );
        }
    };
    println!("curr : {}", &state.curr_ver);
    println!("fetched : {}", &fetched_version);

    if fetched_version == state.curr_ver {
        return json_response_builder(
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse::<Vec<ComponentsList>>::make_success(
                "found components successfuly",
                state.comp_liste.clone(),
            ),
        );
    }

    let _ = match get_new_repo_ver(&repo, &branch, &username) {
        Ok(r) => {
            state.curr_ver = fetched_version;
            r
        }
        Err(err) => {
            println!("Error occured while cloning repo: {} ", err);
            return json_response_builder(
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse::<String>::make_error("Error while checking repo version".to_string()),
            );
        }
    };

    let files_liste = match list_dir_contents(&holding_folder) {
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
        //iterate through the files liste
        .iter()
        .map(|file| -> ComponentsList {
            // read the current file usinf it's path/name
            let file_content = match read_from_file_ut(&format!("{holding_folder}/{file}")) {
                Ok(res) => res,
                Err(err) => {
                    println!("{err}");
                    return ComponentsList {
                        name: file.to_string(),
                        is_legacy: false,
                    };
                }
            };

            let mut is_legacy = false;

            // checking for the presence of the legacy flag
            if file_content.contains("//<legacy") {
                is_legacy = true
            }

            // returning the list elements
            return ComponentsList {
                name: file.to_string(),
                is_legacy,
            };
        })
        // collecting the iterator into a vector (kind of Array)
        .collect();

    state.comp_liste = final_liste.clone();

    // returnig the response
    return json_response_builder(
        StatusCode::INTERNAL_SERVER_ERROR,
        JsonResponse::<Vec<ComponentsList>>::make_success(
            "found components successfuly",
            final_liste,
        ),
    );
}

pub async fn get_component(Path(id): Path<String>) -> impl IntoResponse {
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

    let (_username, repo, _branch) = match extract_repo_info(&config_content) {
        Some(res) => res,
        None => {
            return json_response_builder(
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse::<String>::make_error("could not parse repository url".to_string()),
            );
        }
    };

    let file_path = format!("/etc/compo-doc/tmp/{repo}/components/{id}");
    println!("{}", file_path);

    let mut component_infos = ComponentModel {
        name: String::new(),
        type_name: String::new(),
        comp_code: String::new(),
        comp_type: String::new(),
        is_legacy: false,
    };

    let code = match read_from_file_ut(&file_path) {
        Ok(re) => re,
        Err(err) => {
            println!("{}", err);
            return json_response_builder(
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse::<String>::make_error("could read file content".to_string()),
            );
        }
    };

    if code.contains("//<legacy") {
        component_infos.is_legacy = true
    }

    let type_name = find_used_type(&code).unwrap().unwrap();

    let mut extractor = TypeExtractor::new(&type_name);
    let typing = extractor.extract_from_str(&code).unwrap();

    component_infos.type_name = type_name.clone();
    component_infos.comp_type = type_name;

    let cleared_imports = merge_recurse(&code, &repo);

    component_infos.comp_code = cleared_imports;

    return json_response_builder(
        StatusCode::OK,
        JsonResponse::<ComponentModel>::make_success(
            "repository saved and reached",
            component_infos,
        ),
    );
}

pub async fn setup_config(
    state: Extension<SharedState>,
    Json(config): Json<ConfigContent>,
) -> impl IntoResponse {
    let mut shared_state = state.write().await;
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

    let (username, repo, branch) = match extract_repo_info(&repo_str) {
        Some(res) => res,
        None => {
            return json_response_builder(
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse::<String>::make_error("could not parse repository url".to_string()),
            );
        }
    };

    let _ = match get_new_repo_ver(&config.repo, &branch, &username) {
        Ok(r) => r,
        Err(err) => {
            println!("Error occured while cloning repo: {} ", err);
            return json_response_builder(
                StatusCode::INTERNAL_SERVER_ERROR,
                JsonResponse::<String>::make_error("Error while checking repo version".to_string()),
            );
        }
    };

    let fetch_version =
        match execute_commande(&format!("git ls-remote {} {:?}", &config.repo, &branch)) {
            Ok(v) => v.trim().split("refs").next().unwrap().to_string(),
            Err(err) => {
                print!("{}", err);
                return json_response_builder(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    JsonResponse::<String>::make_error(
                        "Error while checking repo version".to_string(),
                    ),
                );
            }
        };

    shared_state.curr_ver = fetch_version;

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
