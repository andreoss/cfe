use front::config::Config;

#[tokio::main]
async fn main() {
    let config = Config::from_env();
    let app = front::App::new(&config);
    let listener = match tokio::net::TcpListener::bind(&config.bind_addr).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("front: cannot listen on {}: {error}", config.bind_addr);
            std::process::exit(1);
        }
    };
    println!("front: listening on {}", config.bind_addr);
    if let Err(error) = axum::serve(listener, front::routes::router(app)).await {
        eprintln!("front: {error}");
        std::process::exit(1);
    }
}
