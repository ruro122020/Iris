use axum::{routing::{get, post}, Router};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
  let app = Router::new()
      .route("/health", get(health))
      .route("/on", post(turn_on))
      .route("/off", post(turn_off));

  let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
  println!("iris listening on http://127.0.0.1:3000");
  axum::serve(listener, app).await?;
  Ok(())
}
async fn health() -> &'static str {
  "OK\n"
}

async fn turn_on() -> &'static str{
  "led: on\n"
}

async fn turn_off() -> &'static str{
  "led: off\n"
}

