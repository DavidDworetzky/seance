use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct StatusArgs;

pub async fn run(_args: StatusArgs) -> Result<()> {
    let config = crate::config::schema::Config::load(None)?;
    let store = crate::session::store::SessionStore::load()?;
    let quadrants = store.active_quadrants();

    if quadrants.is_empty() {
        println!("No active spirits.");
        return Ok(());
    }

    for q in &quadrants {
        println!("Q{} [{}] monitor={}", q.quadrant, q.branch, q.monitor);
        for name in q.ordered_agent_names(&config.group) {
            if let Some(state) = q.agents.get(&name) {
                println!("  {}: {:?}", name, state.status);
            }
        }
    }

    Ok(())
}
