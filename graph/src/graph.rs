use std::collections::HashMap;
use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
#[cfg(test)]
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct EdgeData {
    pub weight: Option<f64>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub weight: Option<f64>,
    pub label: Option<String>,
}

#[derive(Debug)]
pub enum GraphError {
    VertexNotFound(String),
    VertexAlreadyExists(String),
    EdgeNotFound(String, String),
    EdgeAlreadyExists(String, String),
    Io(io::Error),
    Parse(String),
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphError::VertexNotFound(v) => write!(f, "vertex '{v}' not found"),
            GraphError::VertexAlreadyExists(v) => write!(f, "vertex '{v}' already exists"),
            GraphError::EdgeNotFound(a, b) => write!(f, "edge '{a}' -> '{b}' not found"),
            GraphError::EdgeAlreadyExists(a, b) => {
                write!(f, "edge '{a}' -> '{b}' already exists")
            }
            GraphError::Io(e) => write!(f, "input/output error: {e}"),
            GraphError::Parse(msg) => write!(f, "file parse error: {msg}"),
        }
    }
}

impl Error for GraphError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            GraphError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for GraphError {
    fn from(e: io::Error) -> Self {
        GraphError::Io(e)
    }
}

#[derive(Debug, Clone)]
pub struct Graph {
    directed: bool,
    adjacency: HashMap<String, HashMap<String, EdgeData>>,
}

impl Graph {
    pub fn new(directed: bool) -> Self {
        Graph {
            directed,
            adjacency: HashMap::new(),
        }
    }

    /// Creates a graph with the given vertices and no edges.
    pub fn with_vertices(directed: bool, vertices: &[&str]) -> Self {
        let mut g = Graph::new(directed);
        for &v in vertices {
            g.add_vertex(v)
                .expect("Graph::with_vertices: names of vertices must be unique");
        }
        g
    }

    /// Creates a graph from a list of edges `(from, to, weight, label)`
    /// and adds vertices automatically.
    pub fn from_edges(
        directed: bool,
        edges: &[(&str, &str, Option<f64>, Option<&str>)],
    ) -> Result<Self, GraphError> {
        let mut g = Graph::new(directed);
        for &(from, to, weight, label) in edges {
            if !g.has_vertex(from) {
                g.add_vertex(from)?;
            }
            if !g.has_vertex(to) {
                g.add_vertex(to)?;
            }
            g.add_edge(from, to, weight, label.map(str::to_string))?;
        }
        Ok(g)
    }

    /// Creates a graph from a file in the format described in `save_to_file`.
    /// First line: `DIRECTED`/`UNDIRECTED`, second line: list of vertices separated by commas,
    /// subsequent lines: edges in the format `from,to,weight,label` (`-` means "absent").
    /// Empty lines and lines beginning with `#` are ignored.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, GraphError> {
        let text = fs::read_to_string(path)?;
        Self::parse(&text)
    }

    fn parse(text: &str) -> Result<Self, GraphError> {
        let mut lines = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'));

        let directed = match lines.next() {
            Some("DIRECTED") => true,
            Some("UNDIRECTED") => false,
            Some(other) => {
                return Err(GraphError::Parse(format!(
                    "expected 'DIRECTED' or 'UNDIRECTED', got: '{other}'"
                )));
            }
            None => return Err(GraphError::Parse("empty file".to_string())),
        };

        let mut graph = Graph::new(directed);

        let vertices_line = lines
            .next()
            .ok_or_else(|| GraphError::Parse("vertices line is empty".to_string()))?;
        for name in vertices_line
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            graph
                .add_vertex(name)
                .map_err(|e| GraphError::Parse(format!("duplicate vertex: {e}")))?;
        }

        for (idx, line) in lines.enumerate() {
            let parts: Vec<&str> = line.split(',').map(str::trim).collect();
            let [from, to, weight_s, label_s] = parts.as_slice() else {
                return Err(GraphError::Parse(format!(
                    "line for edge {}: expected 'from,to,weight,label', got '{line}'",
                    idx + 1
                )));
            };

            let weight = if *weight_s == "-" {
                None
            } else {
                Some(weight_s.parse::<f64>().map_err(|_| {
                    GraphError::Parse(format!(
                        "incorrect weight '{weight_s}' for edge {from}-{to}"
                    ))
                })?)
            };
            let label = if *label_s == "-" {
                None
            } else {
                Some(label_s.to_string())
            };

            graph
                .add_edge(from, to, weight, label)
                .map_err(|e| GraphError::Parse(e.to_string()))?;
        }

        Ok(graph)
    }
}

impl Default for Graph {
    fn default() -> Self {
        Graph::new(false)
    }
}

impl Graph {
    pub fn is_directed(&self) -> bool {
        self.directed
    }

    pub fn vertex_count(&self) -> usize {
        self.adjacency.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edge_list().len()
    }

    pub fn has_vertex(&self, name: &str) -> bool {
        self.adjacency.contains_key(name)
    }

    pub fn has_edge(&self, from: &str, to: &str) -> bool {
        self.adjacency
            .get(from)
            .is_some_and(|neighbors| neighbors.contains_key(to))
    }

    pub fn edge_data(&self, from: &str, to: &str) -> Option<&EdgeData> {
        self.adjacency.get(from)?.get(to)
    }

    pub fn vertices(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self.adjacency.keys().map(String::as_str).collect();
        v.sort_unstable();
        v
    }

    /// Returns the neighbors of a vertex
    pub fn neighbors(&self, name: &str) -> Result<Vec<&str>, GraphError> {
        let neighbors = self
            .adjacency
            .get(name)
            .ok_or_else(|| GraphError::VertexNotFound(name.to_string()))?;
        let mut result: Vec<&str> = neighbors.keys().map(String::as_str).collect();
        result.sort_unstable();
        Ok(result)
    }

    /// Returns the vertices adjacent to both `a` and `b`.
    pub fn common_neighbors(&self, a: &str, b: &str) -> Result<Vec<&str>, GraphError> {
        let a_neighbors = self.neighbors(a)?;
        let b_neighbors: HashSet<&str> = self.neighbors(b)?.into_iter().collect();
        Ok(a_neighbors
            .into_iter()
            .filter(|v| b_neighbors.contains(v))
            .collect())
    }

    /// Returns the degree of a vertex
    pub fn degree(&self, name: &str) -> Result<usize, GraphError> {
        let out_degree = self.neighbors(name)?.len();
        if !self.directed {
            return Ok(out_degree);
        }
        let in_degree = self
            .adjacency
            .values()
            .filter(|neighbors| neighbors.contains_key(name))
            .count();
        Ok(out_degree + in_degree)
    }

    /// Returns all leaf (hanging) vertices (vertices of degree 1).
    pub fn leaves(&self) -> Vec<&str> {
        self.vertices()
            .into_iter()
            .filter(|v| {
                self.degree(v)
                    .expect("vertex from self.vertices() always exists")
                    == 1
            })
            .collect()
    }

    /// Returns a copy of the graph with all odd-degree vertices removed.
    pub fn remove_odd_degree_vertices(&self) -> Self {
        let odd_degree_vertices: Vec<&str> = self
            .vertices()
            .into_iter()
            .filter(|v| {
                self.degree(v)
                    .expect("vertex from self.vertices() always exists")
                    % 2
                    == 1
            })
            .collect();

        let mut result = self.clone();
        for v in odd_degree_vertices {
            result
                .remove_vertex(v)
                .expect("vertex from self.vertices() always exists");
        }
        result
    }

    /// Creates a list of edges from adjacency list.
    pub fn edge_list(&self) -> Vec<Edge> {
        let mut edges = Vec::new();
        for from in self.vertices() {
            let neighbors = &self.adjacency[from];
            let mut to_names: Vec<&String> = neighbors.keys().collect();
            to_names.sort_unstable();
            for to in to_names {
                if !self.directed && from > to.as_str() {
                    // In undirected graph edge (to, from) will be already processed
                    continue;
                }
                let data = &neighbors[to];
                edges.push(Edge {
                    from: from.to_string(),
                    to: to.clone(),
                    weight: data.weight,
                    label: data.label.clone(),
                });
            }
        }
        edges
    }
}

impl Graph {
    pub fn add_vertex(&mut self, name: &str) -> Result<(), GraphError> {
        if self.adjacency.contains_key(name) {
            return Err(GraphError::VertexAlreadyExists(name.to_string()));
        }
        self.adjacency.insert(name.to_string(), HashMap::new());
        Ok(())
    }

    pub fn remove_vertex(&mut self, name: &str) -> Result<(), GraphError> {
        if self.adjacency.remove(name).is_none() {
            return Err(GraphError::VertexNotFound(name.to_string()));
        }
        for neighbors in self.adjacency.values_mut() {
            neighbors.remove(name);
        }
        Ok(())
    }

    pub fn add_edge(
        &mut self,
        from: &str,
        to: &str,
        weight: Option<f64>,
        label: Option<String>,
    ) -> Result<(), GraphError> {
        if !self.has_vertex(from) {
            return Err(GraphError::VertexNotFound(from.to_string()));
        }
        if !self.has_vertex(to) {
            return Err(GraphError::VertexNotFound(to.to_string()));
        }
        if self.has_edge(from, to) {
            return Err(GraphError::EdgeAlreadyExists(
                from.to_string(),
                to.to_string(),
            ));
        }

        let forward = EdgeData {
            weight,
            label: label.clone(),
        };
        self.adjacency
            .get_mut(from)
            .unwrap()
            .insert(to.to_string(), forward);

        if !self.directed && from != to {
            let backward = EdgeData { weight, label };
            self.adjacency
                .get_mut(to)
                .unwrap()
                .insert(from.to_string(), backward);
        }
        Ok(())
    }

    pub fn remove_edge(&mut self, from: &str, to: &str) -> Result<(), GraphError> {
        if !self.has_vertex(from) {
            return Err(GraphError::VertexNotFound(from.to_string()));
        }
        if !self.has_vertex(to) {
            return Err(GraphError::VertexNotFound(to.to_string()));
        }
        if self.adjacency.get_mut(from).unwrap().remove(to).is_none() {
            return Err(GraphError::EdgeNotFound(from.to_string(), to.to_string()));
        }
        if !self.directed && from != to {
            self.adjacency.get_mut(to).unwrap().remove(from);
        }
        Ok(())
    }
}

/// Prints adjacency list in a human-readable format.
impl fmt::Display for Graph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Graph ({}), vertices: {}, edges: {}",
            if self.directed {
                "directed"
            } else {
                "undirected"
            },
            self.vertex_count(),
            self.edge_count(),
        )?;
        for v in self.vertices() {
            let neighbors = &self.adjacency[v];
            if neighbors.is_empty() {
                writeln!(f, "  {v}: (isolated)")?;
                continue;
            }
            let mut to_names: Vec<&String> = neighbors.keys().collect();
            to_names.sort_unstable();
            let items: Vec<String> = to_names
                .into_iter()
                .map(|to| {
                    let d = &neighbors[to];
                    match (d.weight, &d.label) {
                        (None, None) => to.clone(),
                        (Some(w), None) => format!("{to} [weight = {w}]"),
                        (None, Some(l)) => format!("{to} [label = {l}]"),
                        (Some(w), Some(l)) => format!("{to} [weight = {w}, label = {l}]"),
                    }
                })
                .collect();
            writeln!(f, "  {v} -> {}", items.join(", "))?;
        }
        Ok(())
    }
}

impl Graph {
    pub fn print_adjacency_list(&self) {
        print!("{self}");
    }

    /// Exports graph to a file in a human-readable plain text format.
    pub fn save_to_file_text(&self, path: impl AsRef<Path>) -> Result<(), GraphError> {
        fs::write(path, self.to_string())?;
        Ok(())
    }

    /// Serializes graph to a reloadable string in format:
    /// `DIRECTED`/`UNDIRECTED`, then a line with all vertices separated by commas,
    /// then one line per edge: `from,to,weight,label` (`-` = no value).
    fn to_serialized_string(&self) -> String {
        let mut out = String::new();
        out.push_str(if self.directed {
            "DIRECTED\n"
        } else {
            "UNDIRECTED\n"
        });
        out.push_str(&self.vertices().join(","));
        out.push('\n');
        for e in self.edge_list() {
            let weight = e
                .weight
                .map(|w| w.to_string())
                .unwrap_or_else(|| "-".to_string());
            let label = e.label.unwrap_or_else(|| "-".to_string());
            out.push_str(&format!("{},{},{},{}\n", e.from, e.to, weight, label));
        }
        out
    }

    // Exports graph to a file in a reloadable format, described in `to_serialized_string`.
    pub fn save_to_file_serialized(&self, path: impl AsRef<Path>) -> Result<(), GraphError> {
        fs::write(path, self.to_serialized_string())?;
        Ok(())
    }

    /// Same as `save_to_file_serialized`, but writes to any `Write`
    /// (e.g., to reuse logic in module tests).
    #[cfg(test)]
    fn write_reloadable(&self, writer: &mut impl Write) -> io::Result<()> {
        writer.write_all(self.to_serialized_string().as_bytes())
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_graph_is_empty() {
        let g = Graph::default();
        assert_eq!(g.vertex_count(), 0);
        assert!(!g.is_directed());
    }

    #[test]
    fn add_and_remove_vertex() {
        let mut g = Graph::new(true);
        g.add_vertex("A").unwrap();
        assert!(g.has_vertex("A"));
        assert!(matches!(
            g.add_vertex("A"),
            Err(GraphError::VertexAlreadyExists(_))
        ));

        g.remove_vertex("A").unwrap();
        assert!(!g.has_vertex("A"));
        assert!(matches!(
            g.remove_vertex("A"),
            Err(GraphError::VertexNotFound(_))
        ));
    }

    #[test]
    fn undirected_edge_is_symmetric() {
        let mut g = Graph::with_vertices(false, &["A", "B"]);
        g.add_edge("A", "B", Some(2.5), None).unwrap();
        assert!(g.has_edge("A", "B"));
        assert!(g.has_edge("B", "A"));
        assert_eq!(g.edge_list().len(), 1);

        g.remove_edge("B", "A").unwrap();
        assert!(!g.has_edge("A", "B"));
        assert!(!g.has_edge("B", "A"));
    }

    #[test]
    fn directed_edge_is_one_way() {
        let mut g = Graph::with_vertices(true, &["A", "B"]);
        g.add_edge("A", "B", None, None).unwrap();
        assert!(g.has_edge("A", "B"));
        assert!(!g.has_edge("B", "A"));
    }

    #[test]
    fn self_loop_counted_once() {
        let mut g = Graph::with_vertices(false, &["A"]);
        g.add_edge("A", "A", Some(1.0), Some("loop".to_string()))
            .unwrap();
        assert_eq!(g.edge_list().len(), 1);
    }

    #[test]
    fn removing_vertex_removes_incident_edges() {
        let mut g = Graph::from_edges(
            true,
            &[
                ("A", "B", None, None),
                ("B", "C", None, None),
                ("C", "A", None, None),
            ],
        )
        .unwrap();
        g.remove_vertex("B").unwrap();
        assert!(!g.has_vertex("B"));
        assert!(!g.has_edge("A", "B"));
        assert!(!g.has_edge("B", "C"));
        assert!(g.has_edge("C", "A"));
    }

    #[test]
    fn add_edge_rejects_missing_vertices_and_duplicates() {
        let mut g = Graph::with_vertices(true, &["A", "B"]);
        assert!(matches!(
            g.add_edge("A", "X", None, None),
            Err(GraphError::VertexNotFound(_))
        ));
        g.add_edge("A", "B", None, None).unwrap();
        assert!(matches!(
            g.add_edge("A", "B", None, None),
            Err(GraphError::EdgeAlreadyExists(_, _))
        ));
    }

    #[test]
    fn clone_is_a_deep_copy() {
        let mut original = Graph::with_vertices(false, &["A", "B"]);
        original.add_edge("A", "B", Some(1.0), None).unwrap();

        let mut copy = original.clone();
        copy.add_vertex("C").unwrap();

        assert!(!original.has_vertex("C"));
        assert_eq!(original.vertex_count(), 2);
        assert_eq!(copy.vertex_count(), 3);
    }

    #[test]
    fn save_and_reload_roundtrip() {
        let mut original = Graph::from_edges(
            true,
            &[
                ("A", "B", Some(3.0), Some("road".to_string()).as_deref()),
                ("B", "C", None, None),
            ],
        )
        .unwrap();
        original.add_vertex("D").unwrap();

        let mut buf = Vec::new();
        original.write_reloadable(&mut buf).unwrap();
        let text = String::from_utf8(buf).unwrap();

        let reloaded = Graph::parse(&text).unwrap();
        assert_eq!(reloaded.is_directed(), original.is_directed());
        assert_eq!(reloaded.vertices(), original.vertices());
        assert_eq!(reloaded.edge_list(), original.edge_list());
    }

    #[test]
    fn common_neighbors_finds_shared_vertices() {
        let g = Graph::from_edges(
            false,
            &[
                ("A", "C", None, None),
                ("B", "C", None, None),
                ("A", "D", None, None),
                ("B", "E", None, None),
            ],
        )
        .unwrap();
        assert_eq!(g.common_neighbors("A", "B").unwrap(), vec!["C"]);
        assert!(g.common_neighbors("A", "E").unwrap().is_empty());
        assert!(matches!(
            g.common_neighbors("A", "X"),
            Err(GraphError::VertexNotFound(_))
        ));
    }

    #[test]
    fn leaves_undirected_are_degree_one_vertices() {
        let g =
            Graph::from_edges(false, &[("A", "B", None, None), ("B", "C", None, None)]).unwrap();
        assert_eq!(g.leaves(), vec!["A", "C"]);
    }

    #[test]
    fn leaves_directed_use_total_degree() {
        // A -> B -> C: B has out-degree 1 and in-degree 1 (total 2), so it's not a leaf.
        let g = Graph::from_edges(true, &[("A", "B", None, None), ("B", "C", None, None)]).unwrap();
        assert_eq!(g.leaves(), vec!["A", "C"]);
    }

    #[test]
    fn isolated_vertex_is_not_a_leaf() {
        let g = Graph::with_vertices(false, &["A", "B", "C"]);
        assert!(g.leaves().is_empty());
    }

    #[test]
    fn remove_odd_degree_vertices_drops_only_odd_degree_ones() {
        // A-B-C-D-A is a 4-cycle (every vertex has degree 2), plus a pendant
        // edge C-E giving C degree 3 and E degree 1.
        let g = Graph::from_edges(
            false,
            &[
                ("A", "B", None, None),
                ("B", "C", None, None),
                ("C", "D", None, None),
                ("D", "A", None, None),
                ("C", "E", None, None),
            ],
        )
        .unwrap();

        let result = g.remove_odd_degree_vertices();
        assert_eq!(result.vertices(), vec!["A", "B", "D"]);
        assert!(result.has_edge("A", "B"));
        assert!(result.has_edge("D", "A"));
        assert!(!result.has_vertex("C"));
        assert!(!result.has_vertex("E"));
    }

    #[test]
    fn remove_odd_degree_vertices_keeps_vertices_turned_odd_by_removal() {
        // A-B-C path: A and C have degree 1 (odd), B has degree 2 (even).
        // Removing A and C leaves B isolated (degree 0), which stays.
        let g =
            Graph::from_edges(false, &[("A", "B", None, None), ("B", "C", None, None)]).unwrap();

        let result = g.remove_odd_degree_vertices();
        assert_eq!(result.vertices(), vec!["B"]);
        assert_eq!(result.edge_count(), 0);
    }

    #[test]
    fn remove_odd_degree_vertices_keeps_graph_with_only_even_degrees() {
        let g = Graph::from_edges(
            false,
            &[
                ("A", "B", None, None),
                ("B", "C", None, None),
                ("C", "A", None, None),
            ],
        )
        .unwrap();
        let result = g.remove_odd_degree_vertices();
        assert_eq!(result.vertices(), g.vertices());
        assert_eq!(result.edge_list(), g.edge_list());
    }

    #[test]
    fn parse_reports_errors_for_bad_input() {
        assert!(matches!(Graph::parse(""), Err(GraphError::Parse(_))));
        assert!(matches!(
            Graph::parse("DIRECTED\nA,B\nA,B,not-a-number,-\n"),
            Err(GraphError::Parse(_))
        ));
    }
}
