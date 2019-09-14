use crate::problem::Problem;


/// A chain of simplifications.
/// We start from an initial problem,
/// then at each step we can either simplify and get a new problem,
/// or perform one step of speedup and get a new problem
#[derive(Clone)]
pub enum Step<T:Clone>{
    Initial(Problem),
    Simplify((T,Problem)),
    Speedup(Problem)
}

/// A generic simplification strategy should implement this trait.
/// A strategy should provide a list of possible simplifications that can be done starting from the current step,
/// and it should be able to tell if the current state is better than the current best one,
/// and if it makes sense to continue trying the current path.
/// Also, it needs to provide a way to simplify the current problem, given the current simplification.
pub trait Auto : Sized + Copy + Clone{
    type Simplification : Copy + Clone ;
    /// given the current state and the maximum number of labels, returns an iterator over the possible simplifications that can be performed.
    fn simplifications(sequence : &mut Sequence<Self>, maxlabels : usize) -> Box<dyn Iterator<Item=Self::Simplification>>;
    /// given the current state, the current best state, and the maximum number of speedup steps, returns true if the current state is better than the stored best one.
    fn should_yield(sequence : &mut Sequence<Self>, best : &mut Sequence<Self>, maxiter : usize) -> bool;
    /// given the current state, the current best state, and the maximum number of speedup steps, returns true it makes sense to do more speedup steps.
    fn should_continue(sequence : &mut Sequence<Self>, best : &mut Sequence<Self>, maxiter : usize) -> bool;
    /// given a problem and a simplification, return a new problem where the simplification has been performed
    fn simplify(p : &mut Problem, simpl : Self::Simplification) -> Problem;
}

#[derive(Clone)]
pub struct Sequence<T> where T : Auto {
    pub steps : Vec<Step<T::Simplification>>,
    pub speedups : usize,
}

impl<T> Sequence<T> where T : Auto {
    pub fn new(p : Problem) -> Self {
		Self{ steps : vec![Step::Initial(p)], speedups : 0 }
	}

    pub fn current(&self) -> &Problem {
        match self.steps.last().unwrap() {
            Step::Initial(p) => {p},
            Step::Simplify((_,p)) => {p},
            Step::Speedup(p) => {p}
        }
    }

    pub fn current_mut(&mut self) -> &mut Problem {
        match self.steps.last_mut().unwrap() {
            Step::Initial(p) => {p},
            Step::Simplify((_,p)) => {p},
            Step::Speedup(p) => {p}
        }
    }
	

    fn make_printable(&mut self) {
        for step in self.steps.iter_mut() {
            match step {
                Step::Initial(p) => {let _ = p.as_result(); },
                Step::Simplify((_,p)) => {let _ = p.as_result(); },
                Step::Speedup(p) => {let _ = p.as_result(); }
            }
        }
    }

    fn push(&mut self, step : Step<T::Simplification>){
        self.steps.push(step);
    }

    fn pop(&mut self){
        self.steps.pop();
    }

    fn pop_speedup(&mut self){
        self.speedups -= 1;
        self.pop();
    }

    fn push_speedup(&mut self) {
        self.speedups += 1;
        let last = self.current_mut();
        let mut new = last.speedup();
        new.assign_chars();
        self.push(Step::Speedup(new));
    }

    fn push_simplification(&mut self, simpl : T::Simplification ) {
        let last = self.current_mut();
        let new = T::simplify(last,simpl);
        self.push(Step::Simplify((simpl,new)));
    }

    fn pop_simplification(&mut self){
        self.pop();
    }
}

pub struct AutomaticSimplifications<T : Auto> {
    pub sol : Sequence<T>,
    pub best : Sequence<T>,
    pub maxiter : usize,
    pub maxlabels : usize
}

impl<T:Auto> AutomaticSimplifications<T> {
    pub fn new(p : Problem, maxiter : usize, maxlabels : usize) -> Self {

        let sol = Sequence::new(p);
        let best = sol.clone();
        Self { sol, best , maxiter, maxlabels}
    }

    /// internal iterator version of automatic simplification,
    /// each time a better result is found, the closure is called
    #[allow(dead_code)]
    pub fn run<F>(&mut self, mut cb : F) where F : FnMut(&Sequence<T>){
        self.problem(&mut cb);	
    }

    fn problem<F>(&mut self, cb : &mut F) where F : FnMut(&Sequence<T>){
        if T::should_yield(&mut self.sol, &mut self.best, self.maxiter) {
            self.best = self.sol.clone();
            self.best.make_printable();
            cb(&self.best);
        }
        if T::should_continue(&mut self.sol, &mut self.best, self.maxiter) {
            self.simplify(cb);
        }
    }
    fn simplify<F>(&mut self, cb : &mut F) where F : FnMut(&Sequence<T>) {
        if self.sol.current().num_labels() <= self.maxlabels {
            self.sol.push_speedup();
            self.problem(cb);
            self.sol.pop_speedup();
        } else {
            for simpl in T::simplifications(&mut self.sol, self.maxlabels) {
                self.sol.push_simplification(simpl);
                self.simplify(cb);
                self.sol.pop_simplification();
            }
            
        }
    }
}


impl<T:Auto> IntoIterator for AutomaticSimplifications<T> {
    type Item = Sequence<T>;
    type IntoIter = AutomaticSimplificationsIntoIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        AutomaticSimplificationsIntoIterator {
            auto: self,
            stack : vec![State::Problem]
        }
    }
}


/// External iterator version of automatic simplification.
/// This allows to get a proper rust iterator, but the code is ugly,
/// since the recursion needs to be converted to a state machine.
enum State<T:Auto> {
    Problem,
    ProblemAfterCheckYield,
    Simplify,
    SimplifyAfterProblemCall,
    SimplifyAfterSimplifyCall,
    SimplifySimplify(Box<dyn Iterator<Item=T::Simplification>>)
}

pub struct AutomaticSimplificationsIntoIterator<T:Auto> {
    auto : AutomaticSimplifications<T>,
    stack : Vec<State<T>>
}

impl<T:Auto> Iterator for AutomaticSimplificationsIntoIterator<T>  {
    type Item = Sequence<T>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.stack.is_empty() {
                return None;
            }
            match self.stack.last_mut().unwrap() {
                State::Problem => {
                    self.stack.pop();
                    self.stack.push(State::ProblemAfterCheckYield);
                    if T::should_yield(&mut self.auto.sol, &mut self.auto.best, self.auto.maxiter) {
                        self.auto.best = self.auto.sol.clone();
                        self.auto.best.make_printable();
                        return Some(self.auto.best.clone());
                    }
                }
                State::ProblemAfterCheckYield => {
                    self.stack.pop();
                    if T::should_continue(&mut self.auto.sol, &mut self.auto.best, self.auto.maxiter) {
                        self.stack.push(State::Simplify);
                    }
                }
                State::Simplify => {
                    self.stack.pop();
                    if self.auto.sol.current().num_labels() <= self.auto.maxlabels {
                        self.auto.sol.push_speedup();
                        self.stack.push(State::SimplifyAfterProblemCall);
                        self.stack.push(State::Problem);
                    } else {
                        self.stack.push(State::SimplifySimplify(T::simplifications(&mut self.auto.sol, self.auto.maxlabels)));
                    }
                }
                State::SimplifyAfterProblemCall => {
                    self.auto.sol.pop_speedup();
                    self.stack.pop();
                }
                State::SimplifySimplify(iter) => {
                    if let Some(simpl) = iter.next() {
                        self.auto.sol.push_simplification(simpl);
                        self.stack.push(State::SimplifyAfterSimplifyCall);
                        self.stack.push(State::Simplify);
                    } else {
                        self.stack.pop();
                    }
                }
                State::SimplifyAfterSimplifyCall => {
                    self.auto.sol.pop_simplification();
                    self.stack.pop();
                }
            }
            
        }
    }
}