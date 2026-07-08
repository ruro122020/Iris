use axum::{routing::{get, post}, Router};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
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

