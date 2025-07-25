use crate::*;

pub fn process_consumer_actions(
    consumer: &mut consumer::Consumer, 
    _state: &SimState, 
    _decision: &Decision, 
    action: &Action) 
{
    match action {
        Action::Buy { good_id, quantity } => {
            println!("Consumer {:?} decides to buy {} of {}", consumer.id, quantity, good_id);
        }
        Action::Sell { good_id, quantity } => {
            println!("Consumer {:?} decides to sell {} of {}", consumer.id, quantity, good_id);
        }
        Action::Save => {
            println!("\nConsumer {:?} decides to save ${:.2}", consumer.id, consumer.income);
        }
    }
}