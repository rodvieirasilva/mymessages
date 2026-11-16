#[tokio::main]
async fn main() -> anyhow::Result<()> {
    mymessages::run_app().await
}
