use url::Url;
use dotenvy_macro::dotenv;
use super::token::get_token;

fn load_env_config() -> (String, String, String, String, String) {
    let client_id = dotenv!("CLIENT_ID").to_string();
    let redirect_uri = dotenv!("REDIRECT_URI").to_string();
    let scope = dotenv!("SCOPE").to_string();
    let response_type = dotenv!("RESPONSE_TYPE").to_string();
    let state = dotenv!("STATE").to_string();

    (client_id, redirect_uri, scope, response_type, state)
}

#[tauri::command]
pub fn open_oauth_login() -> Result<String, String> {
    let (client_id, redirect_uri, scope, response_type, state) = load_env_config();

    let mut url = Url::parse("https://accounts.slc.net/oauth/authorize")
        .map_err(|e| e.to_string())?;

    url.query_pairs_mut()
        .append_pair("client_id", &client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("scope", &scope)
        .append_pair("response_type", &response_type)
        .append_pair("state", &state);

    Ok(url.as_str().into())
}

#[tauri::command]
pub async fn oauth_is_authenticated() -> bool {
    // Placeholder for actual authentication check logic
    // You can implement this based on your application's needs

    let result = get_token().await;
    match result {
        Ok(token) => {
            println!("Access token found: {}", token);
            return true;
        }
        Err(e) => {
            println!("No access token found: {}", e);
        }
    }

    false
}