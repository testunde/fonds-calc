/// fonds calculator
/// author: testunde
/// created: 17.10.2021
///
/// inspired by https://www.zinsen-berechnen.de/fondsrechner.php
///

#[allow(dead_code)]
#[derive(Copy, Clone)]
enum ProfitType {
    DISTRIBUTING,
    ACCUMULATING,
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
enum FundChangeType {
    PERCENT(f32),
    EUR(f32),
}

impl FundChangeType {
    fn get_value(&self, base_value: &f32) -> f32 {
        match &self {
            FundChangeType::PERCENT(x) => (*base_value) * ((*x) / 100f32),
            FundChangeType::EUR(x) => (*x),
        }
    }
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
struct StartConditions {
    entry_funds: f32,       // [EUR]
    input_rate_yearly: f32, // [EUR]
    input_dynamic: FundChangeType,
    stock_rate: FundChangeType,
    profit_type: ProfitType,
    profit_yearly: FundChangeType,
    runtime: u32,                // [years]
    order_costs: FundChangeType, // netto vs brutto
    management_costs: f32,       // [%]
    depot_costs: FundChangeType,
    tax_rate: f32,                 // [%] (normally 26.375%)
    tax_allowance: f32,            // [EUR] (up tp 801EUR or 1602EUR)
    stock_tax_excepmtion_cut: f32, // [%] (normally 30%)
    base_interest_rate: f32,       // [%]
}

#[derive(Copy, Clone, Debug)]
struct StepTemporaries {
    year: u32,                 // [years]
    funds_beginning: f32,      // [EUR]
    input: f32,                // [EUR]
    order_costs: f32,          // [EUR]
    stock_change: f32,         // [EUR]
    management_costs: f32,     // [EUR]
    profit: f32,               // [EUR]
    pre_lump_sum: f32,         // [EUR] TODO: currently "basis-yield"
    diff_to_pre_lump_sum: f32, // [EUR]
    taxes_on_hold: f32,        // [EUR]
    taxes_on_hold_total: f32,  // [EUR]
    funds_end: f32,            // [EUR]
    taxes_on_sell: f32,        // [EUR]
    funds_after_sell: f32,     // [EUR]
    input_total: f32,          // [EUR]
    profit_total: f32,         // [EUR]
    gain_after_tax: f32,       // [EUR]
}

static START_1: StartConditions = StartConditions {
    entry_funds: 5000.00,
    input_rate_yearly: 1000.00,
    input_dynamic: FundChangeType::PERCENT(15.0),
    stock_rate: FundChangeType::PERCENT(3.0),
    profit_type: ProfitType::DISTRIBUTING,
    profit_yearly: FundChangeType::PERCENT(1.38),
    runtime: 21,
    order_costs: FundChangeType::PERCENT(1.5), // EUR(0.90)
    management_costs: 0.22,
    depot_costs: FundChangeType::EUR(0.00),
    tax_rate: 26.375,
    tax_allowance: 750.00, //750.00,
    stock_tax_excepmtion_cut: 30.0,
    base_interest_rate: 3.4, //0.05,
};

fn main() {
    println!("--- Fonds Calc ---");

    // initial data set prior to year 1 (= year 0)
    let mut year_temp = StepTemporaries {
        year: 0,
        funds_beginning: 0.00,
        input: START_1.entry_funds,
        order_costs: 0.00,
        stock_change: 0.00,
        management_costs: 0.00,
        profit: 0.00,
        pre_lump_sum: 0.00,
        diff_to_pre_lump_sum: 0.00,
        taxes_on_hold: 0.00,
        taxes_on_hold_total: 0.00,
        funds_end: 0.00,
        taxes_on_sell: 0.00,
        funds_after_sell: 0.00,
        input_total: 0.00,
        profit_total: 0.00,
        gain_after_tax: 0.00,
    };
    propagate_year_zero(&mut year_temp);

    output_header();
    for y in 1..=START_1.runtime {
        output_year(&year_temp);
        year_temp.year = y;
        set_input_funds(&mut year_temp);
        propagate(&mut year_temp);
    }
    output_year(&year_temp); // output last year's stats
}

fn set_input_funds(dataset: &mut StepTemporaries) {
    dataset.input = match dataset.year {
        1 => START_1.input_rate_yearly,
        _ => dataset.input + START_1.input_dynamic.get_value(&dataset.input),
    };
    dataset.input_total += dataset.input;
}

fn propagate_year_zero(dataset: &mut StepTemporaries) {
    dataset.order_costs = START_1.order_costs.get_value(&dataset.input); // brutto (TODO: netto version)
    dataset.funds_end = dataset.input - dataset.order_costs;
    dataset.funds_after_sell = dataset.funds_end;
    dataset.input_total = dataset.input;
    dataset.gain_after_tax = dataset.funds_after_sell - dataset.input_total;
}

fn propagate(dataset: &mut StepTemporaries) {
    dataset.funds_beginning = dataset.funds_end;
    //dataset.input (+ _total) already done
    dataset.order_costs = START_1.order_costs.get_value(&dataset.input); // brutto (TODO: netto version)
    let funds_intermediate: f32 = dataset.funds_beginning + dataset.input - dataset.order_costs;

    dataset.profit = START_1.profit_yearly.get_value(&funds_intermediate);
    dataset.stock_change = START_1.stock_rate.get_value(&funds_intermediate);
    match START_1.profit_type {
        ProfitType::DISTRIBUTING => {
            dataset.profit = dataset.profit;
            dataset.stock_change = dataset.stock_change
        } // keep profit as outflow
        ProfitType::ACCUMULATING => {
            dataset.profit = 0f32;
            dataset.stock_change += dataset.profit
        } // profit gets reinvested
    }

    let funds_intermediate2: f32 = funds_intermediate + dataset.profit + dataset.stock_change;
    dataset.management_costs = funds_intermediate2 * (START_1.management_costs / 100f32);
    let funds_intermediate3: f32 = funds_intermediate2 - dataset.management_costs;

    dataset.pre_lump_sum =
        0f32.max(funds_intermediate * (START_1.base_interest_rate / 100f32) * 0.7f32); // 0.7 factor is fixed by law

    dataset.taxes_on_hold = get_taxes_on_hold(dataset);
    dataset.taxes_on_hold_total += dataset.taxes_on_hold;
    dataset.profit_total += dataset.profit;

    // "profit" includes "taxes_on_hold" in this context, as one would get "taxes_on_hold" less profit in the end,
    // ..so "taxes_on_hold" + ("profit" - "taxes_on_hold") get subtracted from total funds
    // NEW: taxes_on_hold will not be substraced from reinvested funds but from depot cash balance
    dataset.funds_end = funds_intermediate3 - dataset.profit;

    dataset.taxes_on_sell = get_taxes_on_sell(dataset);

    dataset.funds_after_sell = dataset.funds_end - dataset.taxes_on_sell;
    dataset.gain_after_tax = dataset.funds_after_sell - dataset.input_total + dataset.profit_total
        - (dataset.taxes_on_hold_total - dataset.taxes_on_hold); //not including current year's "taxes_on_hold"
}

fn get_taxes_on_hold(dataset: &mut StepTemporaries) -> f32 {
    match START_1.profit_type {
        ProfitType::DISTRIBUTING => {
            // dataset.diff_to_pre_lump_sum = 0f32;
            // if dataset.profit < dataset.pre_lump_sum {
            //     dataset.diff_to_pre_lump_sum = dataset.pre_lump_sum - dataset.profit;
            // }
            // let vorabpauschale: f32 = dataset.profit.min(dataset.stock_change);
            // let steuerbetrag: f32 = if dataset.profit > dataset.pre_lump_sum {
            //     dataset.profit
            // } else {
            //     0f32.max(vorabpauschale - dataset.profit)
            // };
            // ((dataset.profit + dataset.diff_to_pre_lump_sum)
            // ((steuerbetrag) * (1f32 - (START_1.stock_tax_excepmtion_cut / 100f32))
            //     - START_1.tax_allowance)
            //     * (START_1.tax_rate / 100f32),
            // ######################################################################
            // let tim_vorab: f32 = dataset.pre_lump_sum - dataset.profit;
            // let tim_percentage: f32 = (1f32 - (START_1.stock_tax_excepmtion_cut / 100f32));
            // match dataset.profit {
            //     d if d > dataset.pre_lump_sum => {
            //         (dataset.profit - START_1.tax_allowance) * tim_percentage
            //     }
            //     d if d == 0f32 => dataset.pre_lump_sum * tim_percentage,
            //     d if d <= dataset.pre_lump_sum => {
            //         (0f32.max(dataset.profit) * tim_percentage + tim_vorab * tim_percentage
            //             - START_1.tax_allowance)
            //             * (START_1.tax_rate / 100f32)
            //     }
            //     _ => 0f32,
            // }
            // ######################################################################
            // let vorab: f32 = match dataset.profit {
            //     _d if dataset.stock_change <= 0f32 => 0f32,
            //     d if d > dataset.pre_lump_sum => 0f32,
            //     d if (d + dataset.stock_change) >= dataset.pre_lump_sum => {
            //         dataset.pre_lump_sum - dataset.profit
            //     }
            //     _ => dataset.stock_change,
            // };
            let vorab: f32 = if dataset.profit > dataset.pre_lump_sum {
                dataset.profit
            } else {
                dataset.stock_change.min(dataset.pre_lump_sum)
            };
            ((vorab.max(0f32) + dataset.profit)
                * (1f32 - (START_1.stock_tax_excepmtion_cut / 100f32))
                - START_1.tax_allowance)
                .max(0f32)
                * (START_1.tax_rate / 100f32)

            // FIXME: does not seem to hold for the case with tax_allowance AND high-enough base_interest_rate AND profit_yearly
        }
        ProfitType::ACCUMULATING => 0.0, // TODO
    }
}

fn get_taxes_on_sell(dataset: &mut StepTemporaries) -> f32 {
    let already_taxed: f32 = (dataset.profit)// + dataset.diff_to_pre_lump_sum)
        * (1f32 - (START_1.stock_tax_excepmtion_cut / 100f32)); // by on_hold

    match START_1.profit_type {
        ProfitType::DISTRIBUTING => (0.0f32).max(
            ((dataset.funds_end - dataset.input_total - dataset.diff_to_pre_lump_sum)
                * (1f32 - (START_1.stock_tax_excepmtion_cut / 100f32))
                - 0.0f32.max(START_1.tax_allowance - already_taxed))
                * (START_1.tax_rate / 100f32),
        ),
        ProfitType::ACCUMULATING => 0.0, // TODO: all prior pre_lump_sum's have to be considered
    }
}

fn output_header() {
    println!(
        "{}\t|{}\t|{}\t\t|{}\t|{}\t|{}\t|{}\t\t|{}\t|{}\t|{}\t|{}\t|{}\t|{}\t|{}\t|{}",
        "year",
        "funds_beginning",
        "input",
        "order_costs",
        "stock_change",
        "management_costs",
        "profit",
        "pre_lump_sum",
        "taxes_on_hold",
        "funds_end",
        "taxes_on_sell",
        "funds_after_sell",
        "input_total",
        "profit_total",
        "gain_after_tax"
    );
}

fn output_year(dataset: &StepTemporaries) {
    // TODO alternatives: .csv export, plotting
    // println!("{:?}", dataset);
    println!(
        "{:3}\t|{:10.2}\t\t|{:10.2}\t|{:10.2}\t|{:10.2}\t|{:10.2}\t\t|{:10.2}\t|{:10.2}\t|{:10.2}\t|{:10.2}\t|{:10.2}\t|{:10.2}\t\t|{:10.2}\t|{:10.2}\t|{:10.2}",
        dataset.year,
        dataset.funds_beginning,
        dataset.input,
        -dataset.order_costs,
        dataset.stock_change,
        -dataset.management_costs,
        -dataset.profit,
        dataset.pre_lump_sum,
        -dataset.taxes_on_hold,
        dataset.funds_end,
        -dataset.taxes_on_sell,
        dataset.funds_after_sell,
        dataset.input_total,
        dataset.profit_total,
        dataset.gain_after_tax
    );
}
