use std::fs;
use std::fs::File;
use std::num::ParseFloatError;
use std::path::Path;
use chrono::{DateTime, Local, NaiveDateTime, ParseError, Timelike};
use plotters::prelude::*;
use serde::Serialize;

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
struct PowerRecord {
    date_time: DateTime<Local>,
    pv_power: f64,
}

#[derive(Clone)]
struct PlotData {
    minutes: u32,
    x: f64,
    pv: f64,
}

#[derive(Serialize)]
struct Data {
    x: f64,
    y: f64,
}
#[derive(Serialize)]
struct PVDiagram {
    pv_data: Vec<Data>,
}

/// Program that takes a mygrid stats file as input and produces a normalized file over
/// the PV production of a sunny day. It also produces a plot file.
fn main() {
    let stats_file = "C:/Develop/mygrid_pv/20250403.csv";
    let pv_diagram_file = "C:/Slask/mygrid_dev/config/pv_diagram.json";
    let pv_plot_file = "C:/Slask/mygrid/pv_diagram.png";

    match get_csv_record(Path::new(stats_file)) {
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
            plt = normalize(plt);
            save_pv_diagram(pv_diagram_file, &plt);
            plot_diagram(pv_plot_file, plt);

        }
        Err(e) => {eprintln!("{:?}", e.0)}
    }
}

/// Normalizes a vector of PlotData to X 0..1 and Y 0..1
///
/// # Arguments
///
/// * 'input' - vector to normalize
fn normalize(input: Vec<PlotData>) -> Vec<PlotData> {
    let mut result: Vec<PlotData> = Vec::new();
    let end = input[input.len()-1].minutes as f64;

    let max_value = input.iter().map(|p| p.pv).fold(0.0, |acc, p| p.max(acc));

    for i in input {
        result.push(PlotData{
            minutes: i.minutes,
            x: i.minutes as f64 / end,
            pv: i.pv / max_value,
        });
    }
    result
}

/// Stretches the part of a vector of PlotData that contains positive PV production to fill
/// an entire day of minutes. It does not interpolate the gaps.
///
/// # Arguments
///
/// * 'input' - vector to stretch
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

/// Interpolate the gaps in the given vector of PlotData
///
/// # Arguments
///
/// * 'input' - vector to interpolate
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

/// Performs one round of simple box smoothing of the input vector of PlotData
///
/// # Arguments
///
/// * 'input' - vector to smooth
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
///
/// # Arguments
///
/// * 'plot_file' - the file to save the plot diagram in
/// * 'plot_data' - the vector of PlotData to plot
fn plot_diagram(plot_file: &str, plot_data: Vec<PlotData>) {
    let root = BitMapBackend::new(plot_file, (1280, 480)).into_drawing_area();
    root.fill(&WHITE).unwrap();
    let mut chart = ChartBuilder::on(&root)
        .caption("PVPower", ("sans-serif", 50).into_font())
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0f64..1.1f64, 0f64..1.5f64).unwrap();

    chart.configure_mesh().draw().unwrap();

    chart
        .draw_series(LineSeries::new(
            plot_data.iter().map(|dp| (dp.x, dp.pv)),
            &RED,
        )).unwrap()
        .label("pvPower")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

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

        let csv_record = PowerRecord {
            date_time,
            pv_power,
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

/// Saves a PVDiagram struct to a json file
///
/// # Arguments
///
/// * 'config_file' - the file to save the PVDiagram struct into
/// * 'input' - the vector of PlotData to save as json
fn save_pv_diagram(config_file: &str, input: &Vec<PlotData>) {
    let mut pv_data: Vec<Data> = Vec::new();
    for i in input {
        pv_data.push(Data{ x: i.x, y: i.pv })
    }
    let pv_diagram = PVDiagram { pv_data };

    let json = serde_json::to_string(&pv_diagram).unwrap();

    let path = Path::new(config_file);
    fs::write(path, json).unwrap();
}
