use reqwest::Client;
use crate::oauth::token::get_token;
use crate::oauth::auth::extract_access_refresh_token_from_response;

pub async fn get_request(client: &Client, url: &str) -> Result<String, String> {

    let mut retry_count = 0;
    let max_retries = 1;

    loop {
        let token = get_token().await.map_err(|e| {
            let msg = format!("No access token found: {}", e);
            eprintln!("{}", msg);
            msg
        })?;

        let response = client
            .get(url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let status = response.status();

        if status.is_success() {
            return response.text().await.map_err(|e| format!("Failed to read response text: {}", e));
        }

        if status == reqwest::StatusCode::UNAUTHORIZED {
            if retry_count >= max_retries {
                return Err("Max retries reached: Still unauthorized after token refresh.".into());
            }

            eprintln!("Unauthorized access - attempting to refresh token...");
            
            extract_access_refresh_token_from_response(client).await?;
            
            retry_count += 1;
            continue; 
        }

        let text = response.text().await.unwrap_or_else(|_| "Failed to read error response".to_string());
        eprintln!("Request failed with status {}: {}", status, text);
        return Err(format!("Request failed with status {}: {}", status, text));
    }
}

pub async fn post_request<P, R>(client: &Client, url: &str, params: &P) -> Result<R, String>
where
    P: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    let mut retry_count = 0;
    let max_retries = 1;

    loop {
        let token = get_token().await.map_err(|e| {
            let msg = format!("No access token found: {}", e);
            eprintln!("{}", msg);
            msg
        })?;

        let response = client
            .post(url)
            .bearer_auth(&token)
            .json(params)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let status = response.status();

        if status.is_success() {
            return response
                .json::<R>()
                .await
                .map_err(|e| format!("Failed to parse JSON: {}", e));
        }

        if status == reqwest::StatusCode::UNAUTHORIZED {
            if retry_count >= max_retries {
                return Err("Max retries reached: Still unauthorized after token refresh.".into());
            }

            eprintln!("Unauthorized access - attempting to refresh token...");
            
            extract_access_refresh_token_from_response(client).await?;
            
            retry_count += 1;
            continue; 
        }

        let text = response.text().await.unwrap_or_else(|_| "Failed to read error response".to_string());
        eprintln!("Request failed with status {}: {}", status, text);
        return Err(format!("Request failed with status {}: {}", status, text));
    }
}

pub async fn put_request<P, R>(client: &Client, url: &str, params: &P) -> Result<R, String>
where
    P: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    let mut retry_count = 0;
    let max_retries = 1;

    loop {
        let token = get_token().await.map_err(|e| {
            let msg = format!("No access token found: {}", e);
            eprintln!("{}", msg);
            msg
        })?;

        let response = client
            .put(url)
            .bearer_auth(&token)
            .json(params)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let status = response.status();

        if status.is_success() {
            return response
                .json::<R>()
                .await
                .map_err(|e| format!("Failed to parse JSON: {}", e));
        }

        if status == reqwest::StatusCode::UNAUTHORIZED {
            if retry_count >= max_retries {
                return Err("Max retries reached: Still unauthorized after token refresh.".into());
            }

            eprintln!("Unauthorized access - attempting to refresh token...");
            
            extract_access_refresh_token_from_response(client).await?;
            
            retry_count += 1;
            continue; 
        }

        let text = response.text().await.unwrap_or_else(|_| "Failed to read error response".to_string());
        eprintln!("Request failed with status {}: {}", status, text);
        return Err(format!("Request failed with status {}: {}", status, text));
    }
}