use crate::db::Database;
use crate::events::Event;
use crate::nntp::NntpClient;
use crate::settings::NntpSettings;
use anyhow::Result;
use tokio::sync::mpsc::Sender;
use tokio::time::{sleep, Duration};
use tracing::{error, info};

pub struct Ingestor {
    settings: NntpSettings,
    db: Database,
    sender: Sender<Event>,
}

impl Ingestor {
    pub fn new(settings: NntpSettings, db: Database, sender: Sender<Event>) -> Self {
        Self {
            settings,
            db,
            sender,
        }
    }

    pub async fn run(&self) -> Result<()> {
        info!(
            "Starting NNTP Ingestor for groups: {:?}",
            self.settings.groups
        );

        loop {
            if let Err(e) = self.process_cycle().await {
                error!("Ingestion cycle failed: {}", e);
            }
            // Poll every 60 seconds for now
            sleep(Duration::from_secs(60)).await;
        }
    }

    async fn process_cycle(&self) -> Result<()> {
        let mut client = NntpClient::connect(&self.settings.server, self.settings.port).await?;

        for group_name in &self.settings.groups {
            let info = client.group(group_name).await?;
            info!(
                "Group {}: estimated count={}, low={}, high={}",
                group_name, info.number, info.low, info.high
            );

            // Placeholder for actual fetching logic:
            // 1. Get last_known_id from DB (or memory)
            // 2. Range: (last_known_id + 1) ..= info.high
            // 3. client.article(id)
            // 4. Send to processing queue
            // 5. Update DB
        }

        client.quit().await?;
        Ok(())
    }
}
