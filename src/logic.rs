use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::hash::Hash;
use std::ops::BitAnd;
use std::ops::BitOr;

#[derive(PartialOrd, Ord, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Symbol(u32);

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Symbol(value) = self;
        write!(f, "{value}")
    }
}

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Clone, Debug)]
struct Sum {
    inner: BTreeSet<Symbol>,
}

impl Sum {
    fn iter(&self) -> impl Iterator<Item = Symbol> + '_ {
        self.inner.iter().cloned()
    }

    fn without(&self, symbol: Symbol) -> Sum {
        let mut inner = self.inner.clone();
        inner.remove(&symbol);
        Sum { inner }
    }

    fn contains(&self, symbol: Symbol) -> bool {
        self.inner.contains(&symbol)
    }

    fn is_subset(&self, other: &Sum) -> bool {
        self.inner.is_subset(&other.inner)
    }

    fn remove(&mut self, symbol: Symbol) {
        self.inner.remove(&symbol);
    }
}

impl Extend<Symbol> for Sum {
    fn extend<I: IntoIterator<Item = Symbol>>(&mut self, iter: I) {
        self.inner.extend(iter);
    }
}

impl fmt::Display for Sum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut sep = "";
        for symbol in self.iter() {
            write!(f, "{sep}{symbol}")?;
            sep = " ";
        }
        Ok(())
    }
}

impl<const N: usize> From<[Symbol; N]> for Sum {
    fn from(symbols: [Symbol; N]) -> Self {
        Sum {
            inner: BTreeSet::from(symbols),
        }
    }
}

impl<'a> BitOr for &'a Sum {
    type Output = Sum;
    fn bitor(self, other: &'a Sum) -> Self::Output {
        Sum {
            inner: &self.inner | &other.inner,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Product(Vec<Sum>);

impl Product {
    fn and_identity() -> Product {
        Product::from([])
    }

    fn or_identity() -> Product {
        Product::from([Sum::from([])])
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn iter(&self) -> impl Iterator<Item = &'_ Sum> {
        self.0.iter()
    }
}

impl<const N: usize> From<[Sum; N]> for Product {
    fn from(sums: [Sum; N]) -> Self {
        Product(Vec::from(sums))
    }
}

impl fmt::Display for Product {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut sep = "";
        for sum in self.0.iter() {
            write!(f, "{sep}{sum}")?;
            sep = " ";
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
    type Output = Product;
    fn bitor(self, other: &'a Product) -> Self::Output {
        Product(
            self.0
                .iter()
                .map(move |a| other.0.iter().map(move |b| a | b))
                .flatten()
                .collect(),
        )
    }
}

#[derive(Debug, Clone)]
pub struct Products {
    products: HashMap<Symbol, Product>,
}

impl Products {
    fn get(&self, symbol: Symbol) -> Option<&Product> {
        self.products.get(&symbol)
    }

    fn iter(&self) -> impl Iterator<Item = (Symbol, &Product)> {
        self.products.iter().map(|(&k, v)| (k, v))
    }

    fn len(&self) -> usize {
        self.iter()
            .map(|(_, product)| product.iter().map(|sum| sum.iter().count()).sum::<usize>())
            .sum()
    }

    /// Whenever a implies B:
    /// a=>(B || ...) == eliminate
    /// Example 1:
    /// if a=>b && b=>c && c=>(d || e) && a=>(d || e || f)
    /// we can eliminate the last sum
    /// we'll go through all terms like the third one and see if any other expressions
    /// are like the fourth one (4.rhs >= 3.rhs && 4.lhs => 3.lhs)
    /// Example 2:
    /// a => b || c
    /// b => d
    /// c => e
    /// a => d || e || f     can still eliminate this last sum
    fn minimize(&mut self) {
        // a -> (b || C); b->C === a->C
        fn find_redundant(products: &Products) -> Option<(Symbol, usize, Symbol)> {
            products.iter().find_map(|(lhs, product)| {
                product.iter().enumerate().find_map(|(sum_index, ref sum)| {
                    sum.iter()
                        .find(|&s| {
                            let sum = sum.without(s);
                            products.implies(&Sum::from([s]), &sum, None)
                        })
                        .map(|s| (lhs, sum_index, s))
                })
            })
        }

        fn find_thingy(products: &Products) -> Option<(Symbol, usize)> {
            products.iter().find_map(|(lhs, product)| {
                product
                    .iter()
                    .enumerate()
                    .find(|&(b, ref sum)| products.implies(&Sum::from([lhs]), sum, Some((lhs, b))))
                    .map(|(b, _)| (lhs, b))
            })
        }

        while let Some((lhs, sum_index, redundant)) = find_redundant(self) {
            self.products.get_mut(&lhs).unwrap().0[sum_index].remove(redundant);
        }

        while let Some((a, b)) = find_thingy(self) {
            self.products.get_mut(&a).unwrap().0.remove(b);
        }

        for product in self.products.values_mut() {
            product.0.sort();
            product.0.dedup();
        }
    }

    #[cfg(test)]
    fn implies_test(&self, lhs: &Sum, rhs: &Sum) -> bool {
        self.implies(lhs, rhs, None)
    }

    fn implies(&self, lhs: &Sum, rhs: &Sum, disallow: Option<(Symbol, usize)>) -> bool {
        // we return true iff we can find an equivalent lhs that's a subset of rhs
        // because a ⇒ a ∨ b
        let mut seen = HashSet::from([lhs.clone()]);
        let mut heap = Vec::from([lhs.clone()]);

        while let Some(lhs) = heap.pop() {
            if lhs.is_subset(rhs) {
                //                assert!(lhs.iter().all(|l| self.implies(&Sum::from([l]), rhs, None)));
                //                eprintln!("({}) ({})", lhs, rhs);
                return true;
            }

            for sym in lhs.iter() {
                if let Some(product) = self.get(sym) {
                    for (i, sum) in product.iter().enumerate() {
                        let mut child = lhs.clone();
                        child.remove(sym);
                        child.extend(sum.iter());
                        let child_valid = disallow != Some((sym, i))
                            && !seen.contains(&child)
                            && !child.iter().any(|s| {
                                !rhs.contains(s)
                                    && self.get(s).map(Product::is_empty).unwrap_or(true)
                            });
                        if child_valid {
                            seen.insert(child.clone());
                            heap.push(child);
                        }
                    }
                }
            }
        }

        false
    }
}

impl<const N: usize> From<[(Symbol, Product); N]> for Products {
    fn from(products: [(Symbol, Product); N]) -> Self {
        Products {
            products: HashMap::from(products),
        }
    }
}

impl fmt::Display for Products {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (symbol, product) in self.products.iter() {
            writeln!(f, "{symbol}->[{product}]")?;
        }
        Ok(())
    }
}

pub struct Visitor<N> {
    map: HashMap<N, Symbol>,
    next: u32,
}

impl<N: Hash + Eq> Visitor<N> {
    fn symbol(&mut self, node: N) -> Symbol {
        *self.map.entry(node).or_insert_with(|| {
            self.next += 1;
            Symbol(self.next)
        })
    }

    pub fn visit_node(&mut self, node: N) -> Product {
        Product::from([Sum::from([self.symbol(node)])])
    }

    pub fn visit_all<'b, S, I>(&mut self, iter: I) -> Product
    where
        S: IntoProduct<Node = N> + 'b,
        I: IntoIterator<Item = &'b S>,
    {
        iter.into_iter()
            .map(|tree| tree.into_product(self))
            .fold(Product::and_identity(), BitAnd::bitand)
    }

    pub fn visit_any<'b, S, I>(&mut self, iter: I) -> Product
    where
        S: IntoProduct<Node = N> + 'b,
        I: IntoIterator<Item = &'b S>,
    {
        iter.into_iter()
            .map(|tree| tree.into_product(self))
            .fold(Product::or_identity(), |accum, elem| &accum | &elem)
    }
}

pub trait IntoProduct: Sized {
    type Node: Hash + Eq;
    fn into_product(&self, visitor: &mut Visitor<Self::Node>) -> Product;
    fn node(node: &Self::Node) -> Self;
    fn all(trees: Vec<Self>) -> Self;
    fn any(trees: Vec<Self>) -> Self;
}

fn sum_into_tree<N, S>(sum: &Sum, map: &HashMap<Symbol, N>) -> Option<S>
where
    N: Eq + Hash,
    S: IntoProduct<Node = N>,
{
    let mut symbols: Vec<_> = sum.iter().map(|symbol| S::node(&map[&symbol])).collect();
    match symbols.len() {
        0 => None,
        1 => Some(symbols.pop().unwrap()),
        _ => Some(S::any(symbols)),
    }
}

fn product_into_tree<N, S>(product: &Product, map: &HashMap<Symbol, N>) -> Option<S>
where
    N: Eq + Hash,
    S: IntoProduct<Node = N>,
{
    let mut sums = product
        .iter()
        .map(|sum| sum_into_tree(sum, map))
        .collect::<Option<Vec<_>>>()?;
    match sums.len() {
        0 => Some(S::all(Vec::default())),
        1 => Some(sums.pop().unwrap()),
        _ => Some(S::all(sums)),
    }
}

pub fn minimize<'a, 'b, S, M, N>(trees: M) -> impl Iterator<Item = (N, Option<S>)>
where
    'b: 'a,
    N: Eq + Hash + Clone,
    M: IntoIterator<Item = (N, &'a S)>,
    S: IntoProduct<Node = N> + 'b,
{
    let mut visitor = Visitor {
        map: HashMap::default(),
        next: 0,
    };
    let products = trees
        .into_iter()
        .map(|(node, tree)| (visitor.symbol(node), tree.into_product(&mut visitor)))
        .collect();
    let mut products = Products { products };
    println!("before: {}", products.len());
    products.minimize();
    println!("after: {}", products.len());
    let map: HashMap<Symbol, N> = visitor.map.into_iter().map(|(k, v)| (v, k)).collect();
    products.products.into_iter().map(move |(symbol, product)| {
        let node = map[&symbol].clone();
        let tree = product_into_tree(&product, &map);
        (node, tree)
    })
}

#[cfg(test)]
mod implications {
    use super::Product;
    use super::Products;
    use super::Sum;
    use super::Symbol;

    #[test]
    fn foo() {
        let implications = Products::from([(Symbol(0), Product(vec![Sum::from([Symbol(1)])]))]);
        assert!(implications.implies_test(&Sum::from([Symbol(0)]), &Sum::from([Symbol(1)])));
        assert!(implications.implies_test(&Sum::from([Symbol(0)]), &Sum::from([Symbol(0)])));
        assert!(implications.implies_test(&Sum::from([Symbol(1)]), &Sum::from([Symbol(1)])));
    }

    #[test]
    fn bar() {
        let implications = Products::from([
            (Symbol(0), Product::from([Sum::from([Symbol(1)])])),
            (Symbol(1), Product::from([Sum::from([Symbol(2)])])),
            (Symbol(2), Product::from([Sum::from([Symbol(3)])])),
            (Symbol(3), Product::from([Sum::from([Symbol(4)])])),
            (Symbol(4), Product::from([Sum::from([Symbol(5)])])),
        ]);
        assert!(implications.implies_test(&Sum::from([Symbol(0)]), &Sum::from([Symbol(0)])));
        assert!(implications.implies_test(&Sum::from([Symbol(0)]), &Sum::from([Symbol(1)])));
        assert!(implications.implies_test(&Sum::from([Symbol(1)]), &Sum::from([Symbol(2)])));
        assert!(implications.implies_test(&Sum::from([Symbol(0)]), &Sum::from([Symbol(5)])));

        assert!(!implications.implies_test(&Sum::from([Symbol(1)]), &Sum::from([Symbol(0)])));
        assert!(!implications.implies_test(&Sum::from([Symbol(5)]), &Sum::from([Symbol(0)])));
    }

    #[test]
    fn baz() {
        let implications = Products::from([
            (
                Symbol(0),
                Product::from([Sum::from([Symbol(1), Symbol(2)])]),
            ),
            (Symbol(1), Product::from([Sum::from([Symbol(3)])])),
            (Symbol(2), Product::from([Sum::from([Symbol(3)])])),
        ]);
        assert!(implications.implies_test(&Sum::from([Symbol(0)]), &Sum::from([Symbol(3)])));
        assert!(implications.implies_test(&Sum::from([Symbol(1)]), &Sum::from([Symbol(3)])));
        assert!(implications.implies_test(&Sum::from([Symbol(2)]), &Sum::from([Symbol(3)])));

        assert!(!implications.implies_test(&Sum::from([Symbol(0)]), &Sum::from([Symbol(1)])));
        assert!(!implications.implies_test(&Sum::from([Symbol(3)]), &Sum::from([Symbol(0)])));
    }

    #[test]
    fn qux() {
        let implications = Products::from([
            (
                Symbol(0),
                Product::from([Sum::from([Symbol(1), Symbol(2)])]),
            ),
            (
                Symbol(1),
                Product::from([Sum::from([Symbol(2), Symbol(3), Symbol(4)])]),
            ),
            (Symbol(2), Product::from([Sum::from([Symbol(5)])])),
            (Symbol(3), Product::from([Sum::from([Symbol(5)])])),
            (Symbol(4), Product::from([Sum::from([Symbol(5)])])),
        ]);
        assert!(implications.implies_test(&Sum::from([Symbol(0)]), &Sum::from([Symbol(5)])));

        assert!(!implications.implies_test(&Sum::from([Symbol(2)]), &Sum::from([Symbol(3)])));
    }

    #[test]
    fn quoo() {
        let implications = Products::from([
            (Symbol(0), Product::from([Sum::from([Symbol(1)])])),
            (Symbol(1), Product::from([Sum::from([Symbol(2)])])),
            (Symbol(2), Product::from([Sum::from([Symbol(0)])])),
        ]);
        assert!(implications.implies_test(&Sum::from([Symbol(0)]), &Sum::from([Symbol(1)])));

        assert!(!implications.implies_test(&Sum::from([Symbol(0)]), &Sum::from([Symbol(3)])));
    }

    #[test]
    fn quoo1() {
        let implications = Products::from([
            (Symbol(0), Product::from([Sum::from([Symbol(1)])])),
            (Symbol(1), Product::from([Sum::from([Symbol(2)])])),
            (
                Symbol(2),
                Product::from([Sum::from([Symbol(3)]), Sum::from([Symbol(0)])]),
            ),
        ]);
        assert!(implications.implies_test(&Sum::from([Symbol(0)]), &Sum::from([Symbol(3)])));
        assert!(implications.implies_test(&Sum::from([Symbol(0)]), &Sum::from([Symbol(1)])));

        assert!(!implications.implies_test(&Sum::from([Symbol(3)]), &Sum::from([Symbol(0)])));
    }

    #[test]
    fn quoo2() {
        let implications = Products::from([
            (Symbol(0), Product::from([Sum::from([Symbol(1)])])),
            (Symbol(1), Product::from([Sum::from([Symbol(2)])])),
            (
                Symbol(2),
                Product::from([Sum::from([Symbol(0)]), Sum::from([Symbol(3)])]),
            ),
        ]);
        assert!(implications.implies_test(&Sum::from([Symbol(0)]), &Sum::from([Symbol(3)])));
        assert!(implications.implies_test(&Sum::from([Symbol(0)]), &Sum::from([Symbol(1)])));

        assert!(!implications.implies_test(&Sum::from([Symbol(3)]), &Sum::from([Symbol(0)])));
    }
}
