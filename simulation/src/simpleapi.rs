#![allow(dead_code)]

use crate::auto::AutomaticSimplifications;
use crate::autolb::AutoLb;
use crate::autoub::AutoUb;
use crate::bignum::BigNum;
use crate::problem::DiagramType;
use crate::problem::Problem;
use crate::problem::ResultProblem;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

pub type Simpl = (usize, usize);
pub type Addarrow = (usize, usize);
pub type Renaming = Vec<(Vec<String>, String)>;
pub type Keeping = Vec<String>;
pub type RProblem = (Problem, ResultProblem);
pub type RSimplifications = Vec<(Simpl, (String, String))>;
pub type RLowerBoundStep = Vec<(Problem, crate::autolb::ResultStep, ResultProblem)>;
pub type RUpperBoundStep = Vec<(Problem, crate::autoub::ResultStep, ResultProblem)>;

pub fn new_problem(left: &str, right: &str) -> Result<RProblem, String> {
    let p = Problem::from_text(left, right)?;
    let r = p.as_result();
    Ok((p, r))
}

pub fn speedup(p: &Problem) -> Result<RProblem, String> {
    let np = p.speedup(DiagramType::Accurate)?;
    let nr = np.as_result();
    Ok((np, nr))
}

pub fn possible_simplifications(p: &Problem) -> RSimplifications {
    let pdiag = p.diagram.iter().cloned().chain(p.unreachable_pairs());
    let map = p.map_label_text();
    let rdiag = pdiag.clone().map(|(a,b)|(map[&a].to_owned(),map[&b].to_owned()));
    pdiag.zip(rdiag).collect()
}


pub fn possible_addarrow(p: &Problem) -> RSimplifications {
    let pdiag = p.unreachable_pairs().into_iter();
    let map = p.map_label_text();
    let rdiag = pdiag.clone().map(|(a,b)|(map[&a].to_owned(),map[&b].to_owned()));
    pdiag.zip(rdiag).collect()
}

pub fn simplify(p: &Problem, (a, b): Simpl) -> RProblem {
    let np = p.replace(a, b, DiagramType::Accurate);
    let nr = np.as_result();
    (np, nr)
}

pub fn addarrow(p: &Problem, (a, b): Addarrow) -> RProblem {
    let np = p.relax_add_arrow(a, b, DiagramType::Accurate);
    let nr = np.as_result();
    (np, nr)
}

pub fn harden(p: &Problem, v: Keeping, usepred: bool) -> Result<RProblem, String> {
    let map = &p.map_text_label;
    let map = Problem::map_to_hashmap(map);
    let keep = v
        .iter()
        .map(|x| BigNum::one() << map[x])
        .fold(BigNum::zero(), |a, b| a | b);
    let np = p.harden(keep, DiagramType::Accurate, usepred);
    np.map(|np| {
        let nr = np.as_result();
        (np, nr)
    })
    .ok_or("The new problem would have empty constraints!".into())
}

pub fn rename(p: &Problem, v: Renaming) -> Result<RProblem, String> {
    let newlabelscount = v.iter().map(|(_, s)| s.to_owned()).unique().count();
    if newlabelscount != v.len() {
        return Err("Labels must be different!".into());
    }
    for (_, lab) in &v {
        let valid1 = lab.len() == 1 && lab != "(" && lab != ")";
        let valid2 = lab.len() > 1
            && lab.starts_with("(")
            && lab.ends_with(")")
            && lab.chars().filter(|&x| x == ')').count() == 1;
        if !valid1 && !valid2 {
            return Err(format!("Labels must be either single characters, or strings contained in parentheses! Wrong label: {}",lab));
        }
    }

    let map_text_oldlabel = p.map_text_oldlabel.as_ref().unwrap();
    let map_label_oldset = p.map_label_oldset.as_ref().unwrap();

    let text_to_oldlabel = Problem::map_to_hashmap(map_text_oldlabel);
    let oldset_to_label = Problem::map_to_inv_hashmap(map_label_oldset);

    let oldlabelset_to_oldset = |v: Vec<String>| {
        v.into_iter()
            .map(|s| BigNum::one() << text_to_oldlabel[&s])
            .fold(BigNum::zero(), |a, b| a | b)
    };
    let newmapping = v
        .into_iter()
        .map(|(set, newtext)| {
            let oldset = oldlabelset_to_oldset(set);
            let label = oldset_to_label[&oldset];
            (newtext, label)
        })
        .collect();
    let mut np = p.clone();
    np.map_text_label = newmapping;
    let nr = np.as_result();
    Ok((np, nr))
}

pub fn autolb(
    p: &Problem,
    maxiter: usize,
    maxlabels: usize,
    colors: usize,
    unreach : bool
) -> impl Iterator<Item = Result<RLowerBoundStep, String>> {
    let mut features = vec![];
    if unreach {
        features.push("unreach");
    }
    let auto = AutomaticSimplifications::<AutoLb>::new(p.clone(), maxiter, maxlabels, colors, &features);
    auto.into_iter().map(move |r| {
        r.map(|seq| {
            seq.as_result()
                .steps
                .into_iter()
                .map(|s| {
                    let r = s.1.as_result();
                    (s.1, s.0, r)
                })
                .collect()
        })
    })
}

pub fn autoub(
    p: &Problem,
    maxiter: usize,
    maxlabels: usize,
    colors: usize,
    usepred: bool,
    usedet: bool,
) -> impl Iterator<Item = Result<RUpperBoundStep, String>> {
    let mut features = vec![];
    if usepred {
        features.push("pred");
    }
    if usedet {
        features.push("det");
    }
    let auto =
        AutomaticSimplifications::<AutoUb>::new(p.clone(), maxiter, maxlabels, colors, &features);
    auto.into_iter().map(move |r| {
        r.map(|seq| {
            seq.as_result()
                .steps
                .into_iter()
                .map(|s| {
                    let r = s.1.as_result();
                    (s.1, s.0, r)
                })
                .collect()
        })
    })
}

#[derive(Deserialize, Serialize, Debug)]
pub enum Request {
    NewProblem(String, String),
    Speedup(Problem),
    PossibleSimplifications(Problem),
    PossibleAddarrow(Problem),
    Simplify(Problem, Simpl),
    Addarrow(Problem, Addarrow),
    Harden(Problem, Keeping, bool),
    Rename(Problem, Renaming),
    AutoLb(Problem, usize, usize, usize, bool),
    AutoUb(Problem, usize, usize, usize, bool, bool),
    Ping,
}

#[derive(Deserialize, Serialize)]
pub enum Response {
    Done,
    Pong,
    P(RProblem),
    S(RSimplifications),
    L(RLowerBoundStep),
    U(RUpperBoundStep),
    E(String),
}

pub fn request<F>(req: Request, mut f: F)
where
    F: FnMut(Response),
{
    match req {
        Request::Ping => {
            f(Response::Pong);
            return;
        }
        Request::NewProblem(s1, s2) => match new_problem(&s1, &s2) {
            Ok(r) => f(Response::P(r)),
            Err(s) => f(Response::E(s)),
        },
        Request::Speedup(p) => match speedup(&p) {
            Ok(r) => f(Response::P(r)),
            Err(s) => f(Response::E(s)),
        },
        Request::PossibleSimplifications(p) => {
            let r = possible_simplifications(&p);
            f(Response::S(r));
        }
        Request::PossibleAddarrow(p) => {
            let r = possible_addarrow(&p);
            f(Response::S(r));
        }
        Request::Simplify(p, s) => {
            let r = simplify(&p, s);
            f(Response::P(r));
        }
        Request::Addarrow(p, s) => {
            let r = addarrow(&p, s);
            f(Response::P(r));
        }
        Request::Harden(p, k, x) => match harden(&p, k, x) {
            Ok(r) => f(Response::P(r)),
            Err(s) => f(Response::E(s)),
        },
        Request::Rename(p, x) => match rename(&p, x) {
            Ok(r) => f(Response::P(r)),
            Err(s) => f(Response::E(s)),
        },
        Request::AutoLb(p, i, l, c, u) => {
            for r in autolb(&p, i, l, c, u) {
                match r {
                    Ok(r) => f(Response::L(r)),
                    Err(s) => f(Response::E(s)),
                }
            }
        }
        Request::AutoUb(p, i, l, c, x, y) => {
            for r in autoub(&p, i, l, c, x, y) {
                match r {
                    Ok(r) => f(Response::U(r)),
                    Err(s) => f(Response::E(s)),
                }
            }
        }
    }
    f(Response::Done);
}

pub fn request_json<F>(req: &str, mut f: F)
where
    F: FnMut(String),
{
    let req: Request = serde_json::from_str(req).unwrap();
    let handler = |resp: Response| {
        let s = serde_json::to_string(&resp).unwrap();
        f(s);
    };
    request(req, handler);
}
