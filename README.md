Economic Simulation (Agent-Based)
An extensible economic simulation that models a real economy with interbank and credit markets, goods/services markets, labour, fiscal agents, and robust settlement processes. The system is built on Agent-Based Modelling (ABM): each agent selects actions given its current state, observes outcomes on the next tick, then updates beliefs and knowledge.

Highlights
Multi-market microstructure

Interbank (federal funds–style) lending and Treasuries
Goods & services exchange with order books and history
Labour matching and wage payments
Settlement for interest accrual, coupon payments, and transfers
Agent Decision Models via a unified DecisionModel trait; the engine queries every agent each tick and executes their proposed actions.

Pluggable Domains (Banking, Trading, Production, Consumption, Fiscal, Labour, Settlement) registered dynamically through an inventory-based DomainRegistry.