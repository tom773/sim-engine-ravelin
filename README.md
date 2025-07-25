# Economic, Financial, and Market Simulation Engine for Ravelin Analytics

## Purpose

## How to use

### Training a Decision Model via CLI

1. ```bash git clone git@github.com:tom773/sim-engine-ravelin.git```
2. Navigate to the [Public Use Microdata Files](https://www.bls.gov/cex/pumd_data.htm) page on the BLS website, copy the downlaod link for an interview ZIP file
3. ```bash mkdir ./crates/ml/data && wget <link> -o ./crates/ml/data/``` - make sure the csv files are in this folder, and not in a subsequent ZIP file.
4. Train: ```bash cargo run --bin ml -- train```
5. Validate: ```bash cargo run --bin ml --predict```
6. Predict: ```bash cargo run --bin ml --predict```

ce_train_model.bin should now be in the project root, and can now be used for consumer spending decisions

### Running a Simulation

1. Run the CLI ```bash cargo run --bin cli```. This is a self contained API.
2. View API docs for details on how to run a simulation.