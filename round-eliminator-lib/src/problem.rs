use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

use crate::constraint::Constraint;
use itertools::Itertools;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Problem {
    pub active: Constraint,
    pub passive: Constraint,
    pub mapping_label_text: Vec<(usize, String)>,
    pub mapping_label_oldlabels: Option<Vec<(usize, Vec<usize>)>>,
    pub mapping_oldlabel_text: Option<Vec<(usize, String)>>,
    pub trivial_sets: Option<Vec<Vec<usize>>>,
    pub coloring_sets: Option<Vec<Vec<usize>>>,
    pub diagram_indirect: Option<Vec<(usize, usize)>>,
    pub diagram_indirect_old: Option<Vec<(usize, usize)>>,
    pub diagram_direct: Option<(Vec<(usize, Vec<usize>)>, Vec<(usize, usize)>)>,
}

impl Problem {
    pub fn from_string<S: AsRef<str>>(s: S) -> Result<Self, &'static str> {
        let s = s.as_ref();
        let mut lines = s.lines();
        let mut mapping_label_text = HashMap::new();

        let active = lines.by_ref().take_while(|l| !l.is_empty()).join("\n");
        let active = Constraint::parse(active, &mut mapping_label_text)?;

        let passive = lines.take_while(|l| !l.is_empty()).join("\n");
        let passive = Constraint::parse(passive, &mut mapping_label_text)?;

        let mapping_label_text = mapping_label_text
            .into_iter()
            .map(|(a, b)| (b, a))
            .collect();

        let p = Problem {
            active,
            passive,
            mapping_label_text,
            mapping_label_oldlabels: None,
            mapping_oldlabel_text: None,
            trivial_sets: None,
            coloring_sets: None,
            diagram_indirect: None,
            diagram_direct: None,
            diagram_indirect_old: None,
        };
        Ok(p)
    }

    pub fn labels(&self) -> Vec<usize> {
        let mut labels: Vec<_> = self.mapping_label_text.iter().map(|(l, _)| *l).collect();
        labels.sort();
        labels
    }

    pub fn diagram_indirect_to_reachability_adj(&self) -> HashMap<usize, HashSet<usize>> {
        let mut h: HashMap<usize, HashSet<usize>> = HashMap::new();
        for &(a, b) in self
            .diagram_indirect
            .as_ref()
            .expect("diagram required, but still not computed")
        {
            h.entry(a).or_default().insert(b);
        }
        for label in self.labels() {
            h.entry(label).or_default();
        }
        h
    }

    pub fn diagram_indirect_old_to_reachability_adj(&self) -> HashMap<usize, HashSet<usize>> {
        let mut h: HashMap<usize, HashSet<usize>> = HashMap::new();
        for &(a, b) in self
            .diagram_indirect_old
            .as_ref()
            .expect("old diagram required")
        {
            h.entry(a).or_default().insert(b);
        }
        for label in self
            .mapping_oldlabel_text
            .as_ref()
            .unwrap()
            .iter()
            .map(|(x, _)| *x)
        {
            h.entry(label).or_default();
        }
        h
    }

    pub fn diagram_indirect_to_inverse_reachability_adj(&self) -> HashMap<usize, HashSet<usize>> {
        let mut h: HashMap<usize, HashSet<usize>> = HashMap::new();
        for &(a, b) in self
            .diagram_indirect
            .as_ref()
            .expect("diagram required, but still not computed")
        {
            h.entry(b).or_default().insert(a);
        }
        for label in self.labels() {
            h.entry(label).or_default();
        }
        h
    }
}

impl Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mapping = self.mapping_label_text.iter().cloned().collect();
        for line in &self.active.lines {
            write!(f, "{}\n", line.to_string(&mapping))?;
        }
        write!(f, "\n")?;
        for line in &self.passive.lines {
            write!(f, "{}\n", line.to_string(&mapping))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use crate::{problem::Problem, algorithms::event::EventHandler};

    #[test]
    #[should_panic]
    fn parsing_err() {
        let _ = Problem::from_string("AB^5 BC^100 CD^3\nABCD^108\n\nAB CD**").unwrap();
    }

    #[test]
    fn parsing() {
        let p = Problem::from_string("AB^5 BC^100 CD^3\nABCD^108\n\nAB CD").unwrap();
        assert_eq!(format!("{}", p), "ABCD^108\n\nAB CD\n");

        let p = Problem::from_string("A AB*\nC CD*\n\nAB CD").unwrap();
        assert_eq!(format!("{}", p), "A AB*\nC CD*\n\nAB CD\n");
    }

    #[test]
    fn serialize() {
        let mut p = Problem::from_string("M U*\nP*\n\nM UP*\nU*").unwrap();
        let mut eh = EventHandler::null();
        p.compute_triviality(&mut eh);
        p.compute_diagram(&mut eh);
        let serialized = serde_json::to_string(&p).unwrap();
        println!("{}",serialized);

        let mut p = Problem::from_string("A B B\nC D D\n\nAB AB\nCD CD").unwrap();
        let mut eh = EventHandler::null();
        p.compute_triviality(&mut eh);
        p.compute_diagram(&mut eh);
        let serialized = serde_json::to_string(&p).unwrap();
        println!("{}",serialized);

        let mut p = Problem::from_string("A B B\nC D D\n\nAB CD").unwrap();
        let mut eh = EventHandler::null();
        p.compute_triviality(&mut eh);
        p.compute_coloring_solvability(&mut eh);
        p.compute_diagram(&mut eh);
        let serialized = serde_json::to_string(&p).unwrap();
        println!("{}",serialized);


        let mut p = Problem::from_string("A B AB C\n\nAB AB\nC C").unwrap();
        let mut eh = EventHandler::null();
        p.compute_triviality(&mut eh);
        p.compute_coloring_solvability(&mut eh);
        p.compute_diagram(&mut eh);
        let serialized = serde_json::to_string(&p).unwrap();
        println!("{}",serialized);
    }
}