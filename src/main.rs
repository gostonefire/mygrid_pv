use std::fs::File;
use std::num::ParseFloatError;
use std::path::Path;
use chrono::{DateTime, Local, NaiveDateTime, ParseError, Timelike};
use plotters::prelude::*;


/// Latitude of the power plant
const LAT: f64 = 56.22332313734338;

/// Longitude of the power plant
const LONG: f64 = 15.658393416666142;

#[derive(Debug)]
struct CSVError(String);
impl From<std::io::Error> for CSVError {
    fn from(e: std::io::Error) -> Self {CSVError(e.to_string())}
}
impl From<ParseError> for CSVError {
    fn from(e: ParseError) -> Self {CSVError(e.to_string())}
}
impl From<ParseFloatError> for CSVError {
    fn from(e: ParseFloatError) -> Self {CSVError(e.to_string())}
}
#[derive(Debug)]
struct PowerRecord {
    date_time: DateTime<Local>,
    pv_power: f64,
    ld_power: f64
}

#[derive(Clone)]
struct PlotData {
    minutes: u32,
    x: f64,
    pv: f64,
}

fn main() {
    let date = NaiveDateTime::parse_from_str("2025-04-03 00:00", "%Y-%m-%d %H:%M").unwrap()
        .and_local_timezone(Local)
        .unwrap();

    let path_string = format!("C:/Slask/mygrid/{}.csv", date.format("%Y%m%d"));
    let path = Path::new(&path_string);

    match get_csv_record(path) {
        Ok((records, _)) => {
            let mut plot_data: Vec<PlotData> = Vec::new();
            for record in records {
                let data_point = PlotData {
                    minutes: record.date_time.hour() * 60 + record.date_time.minute(),
                    x: 0.0,
                    pv: record.pv_power * 10.0,
                };

                plot_data.push(data_point);
            }
            let mut plt = smooth(plot_data);
            plt = smooth(plt);
            plt = stretch(plt);
            plt = interpolate(plt);
            plot_diagram(plt);
        }
        Err(e) => {eprintln!("{:?}", e)}
    }
}

fn stretch(input: Vec<PlotData>) -> Vec<PlotData> {
    let mut result: Vec<PlotData> = Vec::new();

    for i in input {
        if i.pv > 0.0 {
            result.push(i);
        }
    }
    let start = result[0].minutes as f64;
    let end = result[result.len()-1].minutes as f64;
    let factor = 1439.0 / (end - start);

    result.iter_mut().for_each(|p| {
        let minutes = (p.minutes as f64 - start).max(0.0) * factor;
        p.x = minutes / 60.0;
        p.minutes = minutes.round().max(0.0).min(1439.0) as u32
    });

    result
}

fn interpolate(input: Vec<PlotData>) -> Vec<PlotData> {
    let mut result: Vec<PlotData> = Vec::new();

    for i in 1..input.len() {
        let x1 = input[i-1].minutes as i32;
        if x1 > 600 {
            print!("");
        }
        let x2 = input[i].minutes as i32;
        let y1 = input[i-1].pv;
        let y2 = input[i].pv;
        let k: f64 = (y1 - y2) / (x1 - x2) as f64;
        let m: f64 = y1 - x1 as f64 * k;
        result.push(input[i-1].clone());
        for x in x1+1..x2 {
            result.push(PlotData{
                minutes: x as u32,
                x: x as f64 / 60.0,
                pv: x as f64 * k + m,
            });
        }
    }
    result.push(input[input.len()-1].clone());
    result
}

fn smooth(input: Vec<PlotData>) -> Vec<PlotData> {
    let mut result: Vec<PlotData> = Vec::new();
    result.push(input[0].clone());
    for i in 1..input.len() - 1 {
        result.push(PlotData{
            minutes: input[i].minutes,
            x: input[i].x,
            pv: (input[i-1].pv + input[i].pv + input[i+1].pv) / 3.0,
        });
    }

    result.push(input[input.len() - 1].clone());
    result
}

/// Plots a diagram based on data from PlotData struct
fn plot_diagram(plot_data: Vec<PlotData>) {
    let root = BitMapBackend::new("C:/Slask/mygrid/0.png", (1280, 480)).into_drawing_area();
    root.fill(&WHITE).unwrap();
    let mut chart = ChartBuilder::on(&root)
        .caption("Sun and PVPower", ("sans-serif", 50).into_font())
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0f64..24f64, 0f64..50f64).unwrap();

    chart.configure_mesh().draw().unwrap();

    chart
        .draw_series(LineSeries::new(
            plot_data.iter().map(|dp| (dp.x, dp.pv)),
            &RED,
        )).unwrap()
        .label("pvPower")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));
    /*
        chart
            .draw_series(LineSeries::new(
                plot_data.iter().map(|dp| (dp.x, dp.pv_est)),
                &GREEN,
            )).unwrap()
            .label("pvEst")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &GREEN));
    */
    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw().unwrap();

    root.present().unwrap();


}


/// Opens and read CSV file into a vector of PowerRecord
///
/// # Arguments
///
/// * 'path' - the path to a csv file to read
fn get_csv_record(path: &Path) -> Result<(Vec<PowerRecord>, DateTime<Local>), CSVError> {
    let mut result: Vec<PowerRecord> = Vec::new();

    let file = File::open(path)?;
    let mut rdr = csv::Reader::from_reader(file);

    for record in rdr.records() {
        let string_record = record.map_err(|e| CSVError(e.to_string()))?;

        let dt = string_record.get(0).ok_or(CSVError("Empty date_time".to_string()))?;
        let date_time = NaiveDateTime::parse_from_str(dt, "%Y-%m-%d %H:%M")?
            .and_local_timezone(Local)
            .unwrap();
        let pv_power = string_record.get(1)
            .ok_or(CSVError("Empty pv_power".to_string()))?
            .parse::<f64>()?;
        let ld_power = string_record.get(2)
            .ok_or(CSVError("Empty ld_power".to_string()))?
            .parse::<f64>()?;

        let csv_record = PowerRecord {
            date_time,
            pv_power,
            ld_power,
        };

        result.push(csv_record);
    }

    if result.len() > 0 {
        let date = result[0].date_time;
        Ok((result, date))
    } else {
        Err(CSVError("Empty CSV-file".to_string()))
    }
}


