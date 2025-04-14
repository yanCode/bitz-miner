use colored::Colorize;
use solana_sdk::signature::Signature;
use tabled::Tabled;

#[derive(Tabled)]
pub struct TableData {
    pub key: String,
    pub value: String,
}

#[derive(Clone, Tabled)]
pub struct SoloCollectingData {
    #[tabled(rename = "Signature")]
    pub signature: String,
    #[tabled(rename = "Block")]
    pub block: String,
    #[tabled(rename = "Timestamp")]
    pub timestamp: String,
    #[tabled(rename = "Timing")]
    pub timing: String,
    #[tabled(rename = "Score")]
    pub difficulty: String,
    #[tabled(rename = "Base Reward")]
    pub base_reward: String,
    #[tabled(rename = "Boost Reward")]
    pub boost_reward: String,
    #[tabled(rename = "Total Reward")]
    pub total_reward: String,
    #[tabled(rename = "Status")]
    pub status: String,
}

impl SoloCollectingData {
    pub fn fetching(sig: Signature) -> Self {
        Self {
            signature: sig.to_string(),
            block: "–".to_string(),
            timestamp: "–".to_string(),
            difficulty: "–".to_string(),
            base_reward: "–".to_string(),
            boost_reward: "–".to_string(),
            total_reward: "–".to_string(),
            timing: "–".to_string(),
            status: "Fetching".to_string(),
        }
    }

    pub fn failed() -> Self {
        Self {
            signature: "–".to_string(),
            block: "–".to_string(),
            timestamp: "–".to_string(),
            difficulty: "–".to_string(),
            base_reward: "–".to_string(),
            boost_reward: "–".to_string(),
            total_reward: "–".to_string(),
            timing: "–".to_string(),
            status: "Failed".bold().red().to_string(),
        }
    }
}

#[derive(Clone, Tabled)]
pub struct PoolCollectingData {
    #[tabled(rename = "Signature")]
    pub signature: String,
    #[tabled(rename = "Block")]
    pub block: String,
    #[tabled(rename = "Timestamp")]
    pub timestamp: String,
    #[tabled(rename = "Timing")]
    pub timing: String,
    #[tabled(rename = "Score")]
    pub difficulty: String,
    #[tabled(rename = "Pool Base Reward")]
    pub base_reward: String,
    #[tabled(rename = "Pool Boost Reward")]
    pub boost_reward: String,
    #[tabled(rename = "Pool Total Reward")]
    pub total_reward: String,
    #[tabled(rename = "My Score")]
    pub my_difficulty: String,
    #[tabled(rename = "My Reward")]
    pub my_reward: String,
}
