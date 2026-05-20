use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MarketplaceListing {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub downloads: u32,
    pub rating: f32,
    pub latest_version: String,
}

pub struct MarketplaceClient {
    // Mock database of available listings
    listings: HashMap<String, MarketplaceListing>,
}

impl Default for MarketplaceClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MarketplaceClient {
    pub fn new() -> Self {
        let listings = HashMap::new();
        Self { listings }
    }

    pub async fn fetch_listings(&self) -> Result<Vec<MarketplaceListing>> {
        Ok(self.listings.values().cloned().collect())
    }

    pub async fn search(&self, query: &str) -> Result<Vec<MarketplaceListing>> {
        let q = query.to_lowercase();
        Ok(self
            .listings
            .values()
            .filter(|l| {
                l.name.to_lowercase().contains(&q) || l.description.to_lowercase().contains(&q)
            })
            .cloned()
            .collect())
    }

    pub async fn install_tool(&self, tool_id: &str) -> Result<()> {
        if self.listings.contains_key(tool_id) {
            // Mock installation logic
            Ok(())
        } else {
            Err(anyhow::anyhow!("Tool not found in marketplace"))
        }
    }
}
