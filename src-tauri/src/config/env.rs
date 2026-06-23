use dotenvy_macro::dotenv;

pub fn load_client_id() -> String {
    dotenv!("CLIENT_ID").to_string()
}

pub fn load_redirect_uri() -> String {
    dotenv!("REDIRECT_URI").to_string()
}

pub fn load_scope() -> String {
    dotenv!("SCOPE").to_string()
}

pub fn load_response_type() -> String {
    dotenv!("RESPONSE_TYPE").to_string()
}

pub fn load_state() -> String {
    dotenv!("STATE").to_string()
}

pub fn load_secret_key() -> String {
    dotenv!("SECRET_KEY").to_string()
}

pub fn load_base_uri() -> String {
    dotenv!("BASE_URI").to_string()
}

pub fn load_backend_url() -> String {
    dotenv!("BACKEND_URL").to_string()
}

pub fn load_bastion_ip() -> String {
    dotenv!("BASTION_IP").to_string()
}

pub fn load_guacamole_url() -> String {
    dotenv!("GUACAMOLE_URL").to_string()
}