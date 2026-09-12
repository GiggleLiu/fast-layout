use fast_layout_engine::{compute, layout, Algorithm, Request, Response, StressMethod};
use serde::Serialize;
use std::env;
use std::time::{Duration, Instant};

#[derive(Serialize)]
struct Row {
    method: &'static str,
    algorithm: String,
    nodes: usize,
    edges: usize,
    dim: usize,
    iterations_requested: usize,
    repeats: usize,
    tolerance: f64,
    stress_method: String,
    theta: Option<f64>,
    min_ms: f64,
    median_ms: f64,
    max_ms: f64,
    iterations: usize,
    converged: bool,
    objective: Option<f64>,
    quality_energy: Option<f64>,
}

struct Args {
    sizes: Vec<usize>,
    algorithms: Vec<Algorithm>,
    dim: usize,
    iterations: usize,
    repeats: usize,
    tolerance: f64,
    theta: f64,
    format: String,
    output: Option<String>,
    graph_file: Option<String>,
    stress_method: StressMethod,
}

fn values(value: &str) -> impl Iterator<Item = &str> {
    value.split(',').filter(|x| !x.is_empty())
}

fn parse_algorithm(value: &str) -> Algorithm {
    match value {
        "stress" => Algorithm::Stress,
        "spring" => Algorithm::Spring,
        "spectral" => Algorithm::Spectral,
        "shell" => Algorithm::Shell,
        "buchheim" => Algorithm::Buchheim,
        _ => panic!("unknown algorithm {value}"),
    }
}

fn parse_args() -> Args {
    let mut args = Args {
        sizes: vec![100, 500, 1000],
        algorithms: vec![Algorithm::Stress, Algorithm::Spring, Algorithm::Spectral],
        dim: 2,
        iterations: 100,
        repeats: 3,
        tolerance: 0.0,
        theta: 0.7,
        format: "csv".into(),
        output: None,
        graph_file: None,
        stress_method: StressMethod::Majorization,
    };
    let mut input = env::args().skip(1);
    while let Some(flag) = input.next() {
        if flag == "--bench" {
            continue;
        }
        let value = input
            .next()
            .unwrap_or_else(|| panic!("missing value for {flag}"));
        match flag.as_str() {
            "--sizes" => {
                args.sizes = values(&value)
                    .map(|x| x.parse().expect("invalid size"))
                    .collect()
            }
            "--algorithms" => args.algorithms = values(&value).map(parse_algorithm).collect(),
            "--dim" => args.dim = value.parse().expect("invalid dimension"),
            "--iterations" => args.iterations = value.parse().expect("invalid iteration count"),
            "--repeats" => args.repeats = value.parse().expect("invalid repeat count"),
            "--tolerance" => args.tolerance = value.parse().expect("invalid tolerance"),
            "--theta" => args.theta = value.parse().expect("invalid theta"),
            "--format" => args.format = value,
            "--output" => args.output = Some(value),
            "--graph-file" => args.graph_file = Some(value),
            "--stress-method" => {
                args.stress_method = match value.as_str() {
                    "majorization" => StressMethod::Majorization,
                    "sgd" => StressMethod::Sgd,
                    _ => panic!("unknown stress method {value}"),
                }
            }
            _ => panic!("unknown option {flag}"),
        }
    }
    assert!(
        !args.sizes.is_empty()
            && args.sizes.iter().all(|&n| n > 0)
            && !args.algorithms.is_empty()
            && args.repeats > 0
    );
    assert!(matches!(args.format.as_str(), "csv" | "json"));
    args
}

fn connected_edges(n: usize) -> Vec<[usize; 2]> {
    [1, 7, 31]
        .into_iter()
        .flat_map(|offset| {
            (0..n).map(move |i| {
                let j = (i + offset) % n;
                if i < j {
                    [i, j]
                } else {
                    [j, i]
                }
            })
        })
        .collect()
}

fn request(n: usize, algorithm: Algorithm, args: &Args) -> Request {
    let edges = if let Some(path) = &args.graph_file {
        serde_json::from_slice(&std::fs::read(path).expect("cannot read graph file"))
            .expect("graph file must be a JSON edge list")
    } else {
        match algorithm {
            Algorithm::Shell => Vec::new(),
            Algorithm::Buchheim => (1..n).map(|i| [(i - 1) / 2, i]).collect(),
            _ => connected_edges(n),
        }
    };
    let dim = if matches!(algorithm, Algorithm::Shell | Algorithm::Buchheim) {
        2
    } else {
        args.dim
    };
    let initial = if matches!(algorithm, Algorithm::Stress | Algorithm::Spring) {
        (0..n)
            .map(|i| {
                Some(
                    (0..dim)
                        .map(|d| match d {
                            0 => (i as f64 + 1.0).sin(),
                            1 => (i as f64 + 1.0).cos(),
                            _ => ((d as f64) * (i as f64 + 1.0)).sin(),
                        })
                        .collect(),
                )
            })
            .collect()
    } else {
        Vec::new()
    };
    Request {
        dim,
        iterations: Some(args.iterations),
        tolerance: args.tolerance,
        stress_method: if algorithm == Algorithm::Stress {
            args.stress_method
        } else {
            StressMethod::Sgd
        },
        initial,
        theta: (algorithm == Algorithm::Spring).then_some(args.theta),
        ..Request::new(n, edges, algorithm)
    }
}

fn stats(mut samples: Vec<Duration>) -> (f64, f64, f64) {
    samples.sort_unstable();
    let ms = |d: Duration| d.as_secs_f64() * 1000.0;
    (
        ms(samples[0]),
        ms(samples[samples.len() / 2]),
        ms(*samples.last().unwrap()),
    )
}

fn measure<F>(repeats: usize, mut f: F) -> (Vec<Duration>, Response)
where
    F: FnMut() -> Response,
{
    let _ = f();
    let mut response = None;
    let samples = (0..repeats)
        .map(|_| {
            let start = Instant::now();
            response = Some(f());
            start.elapsed()
        })
        .collect();
    (samples, response.unwrap())
}

fn row(
    method: &'static str,
    request: &Request,
    repeats: usize,
    samples: Vec<Duration>,
    response: Response,
) -> Row {
    let (min_ms, median_ms, max_ms) = stats(samples);
    let quality_energy = (request.algorithm == Algorithm::Spring)
        .then(|| spring_energy(request, &response.positions));
    Row {
        method,
        algorithm: format!("{:?}", request.algorithm).to_lowercase(),
        nodes: request.nodes,
        edges: request.edges.len(),
        dim: request.dim,
        iterations_requested: request.iteration_limit(),
        repeats,
        tolerance: request.tolerance,
        stress_method: format!("{:?}", request.stress_method).to_lowercase(),
        theta: request.theta,
        min_ms,
        median_ms,
        max_ms,
        iterations: response.iterations,
        converged: response.converged,
        objective: response.objective,
        quality_energy,
    }
}

fn spring_energy(request: &Request, positions: &[Vec<f64>]) -> f64 {
    let k = request.c.unwrap_or(2.0) * (4.0 / request.nodes as f64).sqrt();
    let mut energy = 0.0;
    for i in 0..request.nodes {
        for j in i + 1..request.nodes {
            let distance = positions[i]
                .iter()
                .zip(&positions[j])
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                .sqrt()
                .max(1e-15);
            energy -= k * k * distance.ln();
        }
    }
    for &[i, j] in &request.edges {
        if i == j {
            continue;
        }
        let distance = positions[i]
            .iter()
            .zip(&positions[j])
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt();
        energy += distance.powi(3) / (3.0 * k);
    }
    energy
}

fn main() {
    let args = parse_args();
    let mut rows = Vec::new();
    for &algorithm in &args.algorithms {
        for n in args.sizes.clone() {
            let request = request(n, algorithm, &args);
            let (samples, response) = measure(args.repeats, || {
                compute(&request).expect("native compute failed")
            });
            rows.push(row("compute", &request, args.repeats, samples, response));
            let (samples, response) = measure(args.repeats, || {
                let mut encoded = Vec::new();
                ciborium::into_writer(&request, &mut encoded).unwrap();
                let output = layout(&encoded).expect("CBOR layout failed");
                ciborium::from_reader(output.as_slice()).unwrap()
            });
            rows.push(row("cbor", &request, args.repeats, samples, response));
        }
    }
    if args.format == "json" {
        let output = serde_json::to_string_pretty(&rows).unwrap() + "\n";
        if let Some(path) = args.output {
            std::fs::write(path, &output).expect("cannot write benchmark output");
        }
        print!("{output}");
    } else {
        println!("method,algorithm,nodes,edges,dim,iterations_requested,repeats,tolerance,stress_method,theta,min_ms,median_ms,max_ms,iterations,converged,objective,quality_energy");
        for r in rows {
            println!(
                "{},{},{},{},{},{},{},{},{},{},{:.6},{:.6},{:.6},{},{},{},{}",
                r.method,
                r.algorithm,
                r.nodes,
                r.edges,
                r.dim,
                r.iterations_requested,
                r.repeats,
                r.tolerance,
                r.stress_method,
                r.theta.map_or(String::new(), |x| x.to_string()),
                r.min_ms,
                r.median_ms,
                r.max_ms,
                r.iterations,
                r.converged,
                r.objective.map_or(String::new(), |x| x.to_string()),
                r.quality_energy.map_or(String::new(), |x| x.to_string())
            );
        }
    }
}
