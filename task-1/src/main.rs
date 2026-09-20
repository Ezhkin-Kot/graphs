use std::env;
use std::io::{self, BufRead, Write};

use task_1::{Graph, GraphError};

fn main() {
    let mut graph = match env::args().nth(1) {
        Some(path) => match Graph::from_file(&path) {
            Ok(g) => {
                println!("Loaded graph from file '{path}'.");
                g
            }
            Err(e) => {
                eprintln!("Failed to load graph from '{path}': {e}. Using empty graph.");
                Graph::default()
            }
        },
        None => Graph::default(),
    };

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
    println!("  print              - show the adjacency list");
    println!("  edges              - show the edge list");
    println!("  save <path>        - save the graph to a file (can be loaded back)");
    println!("  save-text <path>   - save the adjacency list in a human-readable form");
    println!("  load <path>        - load a graph from a file (replaces the current one)");
    println!("  help, exit / quit");
}
