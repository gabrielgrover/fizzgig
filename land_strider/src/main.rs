use land_strider::startup::run;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let host = "127.0.0.1";
    let port = "3001";

    run(host, port).await
}
