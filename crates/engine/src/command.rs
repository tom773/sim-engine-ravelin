use crate::state::*;

pub enum SimCommand {
    SaveMoney { consumer_idx: usize, amount: f64 },
    Purchase { consumer_idx: usize, firm_idx: usize, good_id: String, quantity: u32 },
}

pub fn process_command(state: &mut SimState, command: SimCommand) {
    match command {
        SimCommand::SaveMoney { consumer_idx, amount } => {
            let consumer = &mut state.consumers[consumer_idx];
            println!("\n[C{} SAVE] ${:.2}", &consumer.id.0.to_string()[0..3], amount);

        }
        
        SimCommand::Purchase { consumer_idx, firm_idx, good_id, quantity } => {
            // For now, just print the transaction
            println!("[C{} BUY] {} x {} @ $10 from Firm {}", 
                     &state.consumers[consumer_idx].id.0.to_string()[0..3], 
                     good_id, 
                     quantity, 
                     &state.firms[firm_idx].id.0.to_string()[0..3]);
            
            // TODO: Implement actual transaction logic
            // This would involve:
            // 1. Check if consumer has enough cash
            // 2. Check if firm has the goods in inventory
            // 3. Transfer money from consumer to firm
            // 4. Transfer goods from firm to consumer
        }
    }
}