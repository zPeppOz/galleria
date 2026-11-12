#![forbid(unsafe_code)]
use galleria_ipc::{
    DEFAULT_SOCKET_PATH,
    client::{Client, ClientError},
};
#[tokio::main]
async fn main() {
    let result = async {
        let mut c = Client::connect(std::path::Path::new(DEFAULT_SOCKET_PATH)).await?;
        c.hello("ipc-probe", "1").await
    }
    .await;
    let code = match result {
        Ok(_) => 0,
        Err(ClientError::PermissionDenied) => 13,
        Err(_) => 1,
    };
    std::process::exit(code);
}
