use std::env;
use std::io::{self, BufRead, Write};

use graph::{Graph, GraphError};

fn main() {
    let mut graph = env::args()
        .nth(1)
        .and_then(|path| match Graph::from_file(&path) {
            Ok(g) => {
                println!("Loaded graph from file '{path}'.");
                Some(g)
            }
            Err(e) => {
                eprintln!("Failed to load graph from '{path}': {e}. Using an empty graph.");
                None
            }
        })
        .unwrap_or_else(|| {
            println!("Creating a new graph.");
            print!("Directed or undirected? [d/u]: ");
            io::stdout().flush().ok();
            let mut line = String::new();
            io::stdin().read_line(&mut line).ok();
            Graph::new(matches!(
                line.trim().to_lowercase().as_str(),
                "d" | "directed"
            ))
        });

    println!("Adjacency list CLI. Type 'help' for the list of commands.");

    let stdin = io::stdin();
    loop {
        print!("> ");
        io::stdout().flush().ok();

        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
            break; // EOF (Ctrl+D)
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();

        match tokens.as_slice() {
            [] => {}
            ["help"] | ["h"] => print_help(),
            ["exit"] | ["quit"] | ["q"] => break,

            ["print"] | ["p"] => graph.print_adjacency_list(),

            ["edges"] | ["e"] => {
                let edges = graph.edge_list();
                if edges.is_empty() {
                    println!("(no edges)");
                }
                for e in edges {
                    println!(
                        "  {} -> {} [weight={}, label={}]",
                        e.from,
                        e.to,
                        e.weight
                            .map(|w| w.to_string())
                            .unwrap_or_else(|| "-".to_string()),
                        e.label.unwrap_or_else(|| "-".to_string()),
                    );
                }
            }

            ["add-vertex", name] | ["av", name] => {
                report(graph.add_vertex(name), || format!("Vertex '{name}' added."))
            }

            ["remove-vertex", name] | ["rv", name] => report(graph.remove_vertex(name), || {
                format!("Vertex '{name}' removed.")
            }),

            ["add-edge", from, to, rest @ ..] | ["ae", from, to, rest @ ..] if rest.len() <= 2 => {
                match parse_weight_label(rest) {
                    Ok((weight, label)) => {
                        report(graph.add_edge(from, to, weight, label.clone()), || {
                            format!("Edge '{from}' -> '{to}' added.")
                        })
                    }
                    Err(msg) => eprintln!("Error: {msg}"),
                }
            }

            ["remove-edge", from, to] | ["re", from, to] => {
                report(graph.remove_edge(from, to), || {
                    format!("Edge '{from}' -> '{to}' removed.")
                })
            }

            ["common-neighbors", a, b] | ["cn", a, b] => match graph.common_neighbors(a, b) {
                Ok(common) if common.is_empty() => {
                    println!("Vertices '{a}' and '{b}' have no common neighbor.")
                }
                Ok(common) => println!(
                    "Vertices '{a}' and '{b}' have {} common neighbor(s): {}",
                    common.len(),
                    common.join(", ")
                ),
                Err(e) => eprintln!("Error: {e}"),
            },

            ["leaves"] | ["lv"] => {
                let leaves = graph.leaves();
                if leaves.is_empty() {
                    println!("(no leaf vertices)");
                } else {
                    println!("Leaf vertices (degree 1): {}", leaves.join(", "));
                }
            }

            ["remove-odd-degree"] | ["rod"] => {
                graph = graph.remove_odd_degree_vertices();
                println!("Odd-degree vertices removed.");
            }

            ["save", path] | ["s", path] | ["w", path] => match graph.save_to_file_serialized(path)
            {
                Ok(()) => println!("Graph saved to '{path}' (reloadable format)."),
                Err(e) => eprintln!("Failed to save: {e}"),
            },

            ["save-text", path] | ["st", path] => match graph.save_to_file_text(path) {
                Ok(()) => println!("Adjacency list saved to '{path}' (human-readable format)."),
                Err(e) => eprintln!("Failed to save: {e}"),
            },

            ["load", path] | ["l", path] => match Graph::from_file(path) {
                Ok(g) => {
                    graph = g;
                    println!("Graph loaded from '{path}'.");
                }
                Err(e) => eprintln!("Failed to load: {e}"),
            },

            _ => println!("Unknown command or wrong number of arguments. See 'help'."),
        }
    }
}

/// Parses the optional `[weight]` and `[weight] [label]` tail of the `add-edge` command.
fn parse_weight_label(rest: &[&str]) -> Result<(Option<f64>, Option<String>), String> {
    match rest {
        [] => Ok((None, None)),
        [weight] => weight
            .parse::<f64>()
            .map(|w| (Some(w), None))
            .map_err(|_| format!("invalid weight '{weight}'")),
        [weight, label] => weight
            .parse::<f64>()
            .map(|w| (Some(w), Some(label.to_string())))
            .map_err(|_| format!("invalid weight '{weight}'")),
        _ => unreachable!("slice length is bounded above by `rest.len() <= 2`"),
    }
}

fn report(result: Result<(), GraphError>, on_ok: impl FnOnce() -> String) {
    match result {
        Ok(()) => println!("{}", on_ok()),
        Err(e) => eprintln!("Error: {e}"),
    }
}

fn print_help() {
    println!("Commands:");
    println!("  add-vertex <name>");
    println!("  remove-vertex <name>");
    println!("  add-edge <from> <to> [weight] [label]");
    println!("  remove-edge <from> <to>");
    println!("  common-neighbors <a> <b> - list vertices adjacent to both <a> and <b>");
    println!("  leaves                   - list leaf vertices (degree 1)");
    println!("  remove-odd-degree        - remove all odd-degree vertices (single pass)");
    println!("  print                    - show the adjacency list");
    println!("  edges                    - show the edge list");
    println!("  save <path>              - save the graph to a file (can be loaded back)");
    println!("  save-text <path>         - save the adjacency list in a human-readable form");
    println!("  load <path>              - load a graph from a file (replaces the current one)");
    println!("  help, exit / quit");
}
