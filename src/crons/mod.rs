mod starrail;
mod genshin;
mod zenless;
mod themis;
mod honkai;

use std::sync::Arc;
use tokio_cron_scheduler::JobScheduler;
use tracing::info;
use crate::db::DatabaseConnections;
use crate::config::Settings;

pub struct Scheduler {
    db: Arc<DatabaseConnections>,
    config: Arc<Settings>,
}

impl Scheduler {
    pub fn new(db: Arc<DatabaseConnections>, config: Arc<Settings>) -> Self {
        Self { db, config }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let sched = JobScheduler::new().await?;

        info!("Initializing game schedulers...");

        starrail::schedule_tasks(&sched, self.db.clone(), self.config.clone()).await?;
        genshin::schedule_tasks(&sched, self.db.clone(), self.config.clone()).await?;
        zenless::schedule_tasks(&sched, self.db.clone(), self.config.clone()).await?;
        themis::schedule_tasks(&sched, self.db.clone(), self.config.clone()).await?;
        honkai::schedule_tasks(&sched, self.db.clone(), self.config.clone()).await?;

        info!("Starting scheduler");
        sched.start().await?;
        info!("Scheduler started successfully");

        Ok(())
    }
}