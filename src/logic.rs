use std::fmt;
use crate::restrictions::CourseCode;
use std::collections::HashMap;
use crate::restrictions::PrerequisiteTree;
use crate::restrictions::Qualification;
use crate::restrictions::Conjunctive;
use std::iter;
use std::ops::Add;
use std::ops::Mul;
use crate::process::Course;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Symbol(u32);

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Symbol(value) = self;
        write!(f, "{value}")
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Implies(Symbol, Symbol);

impl fmt::Display for Implies {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Implies(lhs, rhs) = self;
        write!(f, "{lhs}>{rhs}")
    }
}

#[derive(Debug)]
struct Sum(Vec<Implies>);

impl fmt::Display for Sum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Sum(implications) = self;
        let mut sep = "";
        write!(f, "(")?;
        for implication in implications {
            write!(f, "{sep}{implication}")?;
            sep = " ";
        }
        write!(f, ")")
    }
}

#[derive(Debug)]
pub struct Product(Vec<Sum>);

impl fmt::Display for Product {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Product(sums) = self;
        for sum in sums {
            write!(f, "{sum}\n")?;
        }
        Ok(())
    }
}

impl Mul for Product {
    type Output = Product;
    fn mul(self, other: Product) -> Self::Output {
        let Product(mut lhs) = self;
        let Product(mut rhs) = other;
        lhs.append(&mut rhs);
        Product(lhs)
    }
}

impl<'a> Add for &'a Product {
    type Output = Product;
    fn add(self, other: &'a Product) -> Self::Output {
        let Product(lhs) = self;
        let Product(rhs) = other;
        let mut ret = Vec::with_capacity(lhs.len() * rhs.len());
        ret.extend(
            lhs.iter()
                .map(|Sum(l)| {
                    rhs.iter()
                        .map(|Sum(r)| Sum(l.iter().chain(r.iter()).cloned().collect()))
                })
                .flatten(),
        );
        Product(ret)
    }
}

impl iter::Product for Product {
    fn product<I: Iterator<Item=Product>>(iter: I) -> Product {
        let identity = Product(vec![]);
        iter.fold(identity, Mul::mul)
    }
}

impl iter::Sum<Product> for Product {
    fn sum<I: Iterator<Item=Product>>(iter: I) -> Product {
        let identity = Product(vec![Sum(vec![])]);
        iter.fold(identity, |accum, elem| &accum + &elem)
    }
}

struct SymbolGenerator(u32);

impl SymbolGenerator {
    fn next(&mut self) -> Symbol {
        self.0 += 1;
        Symbol(self.0)
    }
}

impl Product {
    pub fn from_courses(courses: &[Course]) -> (Product, HashMap<Qualification, Symbol>) {
        let mut generator = SymbolGenerator(0);
        let mut map = HashMap::new();

        let mut all_products = Product(vec![]);
        
        for course in courses {
            if let Some(tree) = &course.prerequisites {
                let lhs = *map.entry(Qualification::Course(CourseCode::try_from(course.code.inner.as_str()).unwrap()))
                    .or_insert_with(|| generator.next());
                let product = Product::from_prereq_tree(tree, lhs, &mut generator, &mut map);
                all_products = all_products * product;
            }
        }

        (all_products, map)
    }

    fn from_prereq_tree<'a>(
        tree: &'a PrerequisiteTree, 
        lhs: Symbol,
        generator: &mut SymbolGenerator,
        map: &mut HashMap<Qualification, Symbol>,
    ) -> Self {
        match tree {
            PrerequisiteTree::Qualification(qualification) => {
                let symbol = *map.entry(qualification.clone())
                    .or_insert_with(|| generator.next());
                Product(vec![Sum(vec![Implies(lhs, symbol)])])
            },
            PrerequisiteTree::Conjunctive(conjunctive, children) => {
                let children = children.iter()
                    .map(|t| Product::from_prereq_tree(t, lhs, generator, map));
                match conjunctive {
                    Conjunctive::All => children.product(),
                    Conjunctive::Any => children.sum(),
                }
            },
        }
    }
}
