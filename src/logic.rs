use std::cell::RefCell;
use std::fmt;
use crate::restrictions::CourseCode;
use std::collections::HashMap;
use std::collections::HashSet;
use crate::restrictions::PrerequisiteTree;
use crate::restrictions::Qualification;
use crate::restrictions::Conjunctive;
use std::iter;
use std::ops::BitAnd;
use std::ops::BitOr;
use crate::process::Course;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Symbol(u32);

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Symbol(value) = self;
        write!(f, "{value}")
    }
}

#[derive(Clone, Debug)]
struct Sum {
    inner: HashSet<Symbol>,
}

impl Sum {
    fn iter(&self) -> impl Iterator<Item=Symbol> + '_ {
        self.inner.iter().cloned()
    }

    fn remove(&mut self, symbol: Symbol) {
        self.inner.remove(&symbol);
    }
}

impl<const N: usize> From<[Symbol; N]> for Sum {
    fn from(symbols: [Symbol; N]) -> Self {
        Sum { inner: HashSet::from(symbols) }
    }
}

impl<'a> BitOr for &'a Sum {
    type Output = Sum;
    fn bitor(self, other: &'a Sum) -> Self::Output {
        Sum { inner: &self.inner | &other.inner }
    }
}

#[derive(Clone, Debug)]
struct Implies {
    lhs: Symbol,
    rhs: Sum,
}

impl fmt::Display for Implies {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut sep = "";
        write!(f, "{}->(", self.lhs)?;
        for value in self.rhs.iter() {
            write!(f, "{sep}{value}")?;
            sep = " ";
        }
        write!(f, ")")
    }
}

#[derive(Clone, Debug)]
pub struct Product(Vec<Implies>);

impl Product {
    pub fn size(&self) -> usize {
        self.0.iter().map(|Implies { rhs, .. }| rhs.iter().count()).sum()
    }

    fn and_identity() -> Product {
        Product(vec![])
    }

    fn or_identity(lhs: Symbol) -> Product {
        Product(vec![Implies { lhs, rhs: Sum::from([]) }])
    }
}

impl fmt::Display for Product {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Product(sums) = self;
        for sum in sums {
            write!(f, "{sum}\n")?;
        }
        Ok(())
    }
}

impl BitAnd for Product {
    type Output = Product;
    fn bitand(mut self, mut other: Product) -> Self::Output {
        self.0.append(&mut other.0);
        self
    }
}

impl<'a> BitOr for &'a Product {
    type Output = Option<Product>;
    /// `self` and `other` must have all the same left hand sides
    fn bitor(self, other: &'a Product) -> Self::Output {
        let implications = self.0.iter()
            .map(move |Implies { lhs: l0, rhs: r0 }| {
                other.0.iter()
                    .map(move |Implies { lhs: l1, rhs: r1 }| {
                        (l0 == l1).then(|| Implies {
                            lhs: *l0,
                            rhs: r0 | r1,
                        })
                    })
            })
            .flatten()
            .collect::<Option<Vec<Implies>>>()?;
        Some(Product(implications))
    }
}

#[derive(Default)]
struct SymbolMap {
    map: HashMap<Qualification, Symbol>,
    next: u32,
}

impl SymbolMap {
    fn symbol(&mut self, qualification: Qualification) -> Symbol {
        *self.map.entry(qualification)
            .or_insert_with(|| {
                self.next += 1;
                Symbol(self.next)
            })
    }
}

pub struct Implications {
    product: Product,
    memo: RefCell<HashMap<(Symbol, Symbol), bool>>,
}

impl Implications {
    fn implies(&self, lhs: Symbol, rhs: Symbol) -> bool {
        self.implies_helper(lhs, rhs, &mut HashSet::new())
    }

    fn implies_helper(&self, lhs: Symbol, rhs: Symbol, seen: &mut HashSet<(Symbol, Symbol)>) -> bool {
        if lhs == rhs {
            return true;
        } else if let Some(result) = self.memo.borrow().get(&(lhs, rhs)) {
            return *result;
        }

        if !seen.insert((lhs, rhs)) { // was present?
            return false; // todo: figure out what should be here
        }

        let result = self.product.0.iter()
            .filter(|implies| implies.lhs == lhs) // todo: better data structure
            .any(|implies| implies.rhs.iter().all(|l| self.implies_helper(l, rhs, seen)));

        self.memo.borrow_mut().insert((lhs, rhs), result);
        result
    }
}

impl From<Product> for Implications {
    fn from(product: Product) -> Self {
        Implications { product, memo: RefCell::default() }
    }
}

#[cfg(test)]
mod implications {
    use super::Implications;
    use super::Symbol;
    use super::Sum;
    use super::Product;
    use super::Implies;

    #[test]
    fn foo() {
        let product = Product(vec![
            Implies { lhs: Symbol(0), rhs: Sum::from([Symbol(1)]) },
        ]);
        let implications = Implications::new(product);
        assert!(implications.implies(Symbol(0), Symbol(1)));
        assert!(implications.implies(Symbol(0), Symbol(0)));
        assert!(implications.implies(Symbol(1), Symbol(1)));
    }

    #[test]
    fn bar() {
        let product = Product(vec![
            Implies { lhs: Symbol(0), rhs: Sum::from([Symbol(1)]) },
            Implies { lhs: Symbol(1), rhs: Sum::from([Symbol(2)]) },
            Implies { lhs: Symbol(2), rhs: Sum::from([Symbol(3)]) },
            Implies { lhs: Symbol(3), rhs: Sum::from([Symbol(4)]) },
            Implies { lhs: Symbol(4), rhs: Sum::from([Symbol(5)]) },
        ]);
        let implications = Implications::new(product);
        assert!(implications.implies(Symbol(0), Symbol(0)));
        assert!(implications.implies(Symbol(0), Symbol(1)));
        assert!(implications.implies(Symbol(1), Symbol(2)));
        assert!(implications.implies(Symbol(0), Symbol(5)));
        
        assert!(!implications.implies(Symbol(1), Symbol(0)));
        assert!(!implications.implies(Symbol(5), Symbol(0)));
    }

    #[test]
    fn baz() {
        let product = Product(vec![
            Implies { lhs: Symbol(0), rhs: Sum::from([Symbol(1), Symbol(2)]) },
            Implies { lhs: Symbol(1), rhs: Sum::from([Symbol(3)]) },
            Implies { lhs: Symbol(2), rhs: Sum::from([Symbol(3)]) },
        ]);
        let implications = Implications::new(product);
        assert!(implications.implies(Symbol(0), Symbol(3)));
        assert!(implications.implies(Symbol(1), Symbol(3)));
        assert!(implications.implies(Symbol(2), Symbol(3)));
        
        assert!(!implications.implies(Symbol(0), Symbol(1)));
        assert!(!implications.implies(Symbol(3), Symbol(0)));
    }    

    #[test]
    fn qux() {
        let product = Product(vec![
            Implies { lhs: Symbol(0), rhs: Sum::from([Symbol(1), Symbol(2)]) },
            Implies { lhs: Symbol(1), rhs: Sum::from([Symbol(2), Symbol(3), Symbol(4)]) },
            Implies { lhs: Symbol(2), rhs: Sum::from([Symbol(5)]) },
            Implies { lhs: Symbol(3), rhs: Sum::from([Symbol(5)]) },
            Implies { lhs: Symbol(4), rhs: Sum::from([Symbol(5)]) },
        ]);
        let implications = Implications::new(product);
        assert!(implications.implies(Symbol(0), Symbol(5)));
        
        assert!(!implications.implies(Symbol(2), Symbol(3)));
    }    
}

impl Product {
    pub fn from_courses<'a, I: Iterator<Item=&'a Course>>(courses: I) -> (Product, HashMap<Qualification, Symbol>) {
        let mut map = SymbolMap::default();

        let product = courses
            .filter_map(|course| Some((&course.code, course.prerequisites.as_ref()?)))
            .map(|(code, prerequisites)| {
                let qualification = Qualification::Course(CourseCode::try_from(code.inner.as_str()).unwrap());
                let lhs = map.symbol(qualification);
                Product::from_prereq_tree(prerequisites, lhs, &mut map)
            })
            .fold(Product::and_identity(), BitAnd::bitand);

        (product, map.map)
    }

    fn from_prereq_tree<'a>(
        tree: &'a PrerequisiteTree, 
        lhs: Symbol,
        map: &mut SymbolMap,
    ) -> Self {
        match tree {
            PrerequisiteTree::Qualification(qualification) => {
                let symbol = map.symbol(qualification.clone());
                Product(vec![Implies { lhs, rhs: Sum::from([symbol]) }])
            },
            PrerequisiteTree::Conjunctive(conjunctive, children) => {
                let mut children = children.iter()
                    .map(|t| Product::from_prereq_tree(t, lhs, map));
                match conjunctive {
                    Conjunctive::All => children.fold(Product::and_identity(), BitAnd::bitand),
                    Conjunctive::Any => children.try_fold(Product::or_identity(lhs), |accum, elem| &accum | &elem).unwrap(),
                }
            },
        }
    }

    pub fn minimize(&mut self) {
        let implications = Implications::from(self.clone());
        self.stage1(&implications);
        self.stage2(&implications);
    }

    /// a=>b && c=>(a || b || ...) == a=>b && c=>(b || ...)
    fn stage1(&mut self, implications: &Implications) {
        for Implies { rhs, .. } in self.0.iter_mut() {
            let found =  rhs.iter()
                .find(|&l| rhs.iter().any(|r| l != r && implications.implies(l, r)));

            if let Some(found) = found {
                rhs.remove(found);
            }
        }
    }

    /// a=>b && a=>(b || ...) == a=>b
    fn stage2(&mut self, implications: &Implications) {
        self.0.retain(|Implies { lhs, rhs }| {
            rhs.iter().all(|r| !implications.implies(*lhs, r))
        })
    }
}


