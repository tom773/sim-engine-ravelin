pub mod macros;
pub mod pricing;
pub mod time;

pub use pricing::*;
pub use time::*;

use crate::*;
use std::any::Any;
pub fn get_agent_display_name(agent: &dyn Any, agent_id: &AgentId) -> String {
    if agent.is::<Bank>() {
        format!("Bank-{}", &agent_id.to_string()[..4])
    } else if agent.is::<Firm>() {
        format!("Firm-{}", &agent_id.to_string()[..4])
    } else if agent.is::<Consumer>() {
        format!("Consumer-{}", &agent_id.to_string()[..4])
    } else if agent.is::<Government>() {
        "Government".to_string()
    } else if agent.is::<CentralBank>() {
        "Central Bank".to_string()
    } else {
        "Unknown Agent".to_string()
    }
}
