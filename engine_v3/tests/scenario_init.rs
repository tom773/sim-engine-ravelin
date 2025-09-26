use engine_v3::Scenario;

#[test]
fn initialize_config_scenario() {
    let scenario_str = include_str!("../../config/config.toml");
    let scenario = Scenario::from_toml_str(scenario_str).expect("parse scenario");
    scenario.initialize_engine();
}
