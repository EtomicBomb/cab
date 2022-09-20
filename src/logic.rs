use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::hash::Hash;
use std::ops::BitAnd;
use std::ops::BitOr;

pub trait Symbol: Ord + Eq + Hash + Clone {
    fn cmp_rank(&self, other: &Self) -> Option<Ordering>;

    fn ge(&self, other: &Self) -> bool {
        self.cmp_rank(other).map(Ordering::is_ge).unwrap_or(false)
    }
}

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Clone, Debug)]
struct Sum<S> {
    inner: BTreeSet<S>,
}

impl<S: Symbol> Sum<S> {
    fn iter(&self) -> impl Iterator<Item = &'_ S> {
        self.inner.iter()
    }

    fn into_iter(self) -> impl Iterator<Item = S> {
        self.inner.into_iter()
    }

    fn without(&self, symbol: &S) -> Sum<S> {
        let mut inner = self.inner.clone();
        inner.remove(symbol);
        Sum { inner }
    }

    fn contains(&self, symbol: &S) -> bool {
        self.inner.contains(symbol)
    }

    fn difference<'a>(&'a self, other: &'a Sum<S>) -> impl Iterator<Item = &S> {
        self.inner.difference(&other.inner)
    }

    fn is_subset(&self, other: &Sum<S>) -> bool {
        self.inner.is_subset(&other.inner)
    }

    fn remove(&mut self, symbol: &S) {
        self.inner.remove(symbol);
    }
}

impl<S: Symbol> Extend<S> for Sum<S> {
    fn extend<I: IntoIterator<Item = S>>(&mut self, iter: I) {
        self.inner.extend(iter);
    }
}

impl<const N: usize, S: Symbol> From<[S; N]> for Sum<S> {
    fn from(symbols: [S; N]) -> Self {
        Sum {
            inner: BTreeSet::from(symbols),
        }
    }
}

impl<'a, S: Symbol> BitOr for &'a Sum<S> {
    type Output = Sum<S>;
    fn bitor(self, other: &'a Sum<S>) -> Self::Output {
        Sum {
            inner: &self.inner | &other.inner,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Product<S>(Vec<Sum<S>>);

impl<S: Symbol> Product<S> {
    fn and_identity() -> Product<S> {
        Product::from([])
    }

    fn or_identity() -> Product<S> {
        Product::from([Sum::from([])])
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn iter(&self) -> impl Iterator<Item = &'_ Sum<S>> {
        self.0.iter()
    }

    fn into_iter(self) -> impl Iterator<Item = Sum<S>> {
        self.0.into_iter()
    }
}

impl<const N: usize, S: Symbol> From<[Sum<S>; N]> for Product<S> {
    fn from(sums: [Sum<S>; N]) -> Self {
        Product(Vec::from(sums))
    }
}

impl<S> BitAnd for Product<S> {
    type Output = Product<S>;
    fn bitand(mut self, mut other: Product<S>) -> Self::Output {
        self.0.append(&mut other.0);
        self
    }
}

impl<'a, S: Symbol> BitOr for &'a Product<S> {
    type Output = Product<S>;
    fn bitor(self, other: &'a Product<S>) -> Self::Output {
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
pub struct Products<S> {
    products: HashMap<S, Product<S>>,
}

impl<S: Symbol> Products<S> {
    fn get(&self, symbol: &S) -> Option<&Product<S>> {
        self.products.get(symbol)
    }

    fn iter(&self) -> impl Iterator<Item = (&S, &Product<S>)> {
        self.products.iter()
    }

    fn len(&self) -> usize {
        self.iter()
            .map(|(_, product)| product.iter().map(|sum| sum.iter().count()).sum::<usize>())
            .sum()
    }

    fn find_redundant(&self) -> Option<(S, usize, S)> {
        self.iter().find_map(|(lhs, product)| {
            product.iter().enumerate().find_map(|(sum_index, ref sum)| {
                sum.iter()
                    .find(|&s| {
                        let sum = sum.without(s);
                        self.implies(&Sum::from([s.clone()]), &sum, None)
                    })
                    .map(|s| (lhs.clone(), sum_index, s.clone()))
            })
        })
    }

    fn find_thingy(&self) -> Option<(S, usize)> {
        self.iter().find_map(|(lhs, product)| {
            product
                .iter()
                .enumerate()
                .find(|&(b, ref sum)| self.implies(&Sum::from([lhs.clone()]), sum, Some((&lhs, b))))
                .map(|(b, _)| (lhs.clone(), b))
        })
    }

    fn minimize(&mut self) {
        // a -> (b || C); b->C === a->C

        while let Some((lhs, sum_index, redundant)) = self.find_redundant() {
            self.products.get_mut(&lhs).unwrap().0[sum_index].remove(&redundant);
        }

        while let Some((a, b)) = self.find_thingy() {
            self.products.get_mut(&a).unwrap().0.remove(b);
        }

        for product in self.products.values_mut() {
            product.0.sort();
            product.0.dedup();
        }
    }

    #[cfg(test)]
    fn implies_test(&self, lhs: &Sum<S>, rhs: &Sum<S>) -> bool {
        self.implies(lhs, rhs, None)
    }

    fn implies(&self, lhs: &Sum<S>, rhs: &Sum<S>, disallow: Option<(&S, usize)>) -> bool {
        // we return true iff we can find an equivalent lhs that's a subset of rhs
        // because a ⇒ a ∨ b
        let mut seen = HashSet::from([lhs.clone()]);
        let mut heap = Vec::from([lhs.clone()]);
        while let Some(lhs) = heap.pop() {
            let is_subset = lhs.difference(&rhs).all(|l| {
                rhs.iter()
                    .any(|r| l.cmp_rank(r).map(Ordering::is_ge).unwrap_or(false))
            });
            if is_subset {
                return true;
            }
            for sym in lhs.iter() {
                if let Some(product) = self.get(sym) {
                    for (i, sum) in product.iter().enumerate() {
                        let mut child = lhs.clone();
                        child.remove(sym);
                        child.extend(sum.iter().cloned());
                        let child_valid = disallow != Some((sym, i))
                            && !seen.contains(&child)
                            && !child.iter().any(|s| {
                                !rhs.iter()
                                    .any(|r| s.cmp_rank(r).map(Ordering::is_ge).unwrap_or(false))
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

impl<const N: usize, S: Symbol> From<[(S, Product<S>); N]> for Products<S> {
    fn from(products: [(S, Product<S>); N]) -> Self {
        Products {
            products: HashMap::from(products),
        }
    }
}

pub fn visit_symbol<S: Symbol>(symbol: S) -> Product<S> {
    Product::from([Sum::from([symbol])])
}

pub fn visit_all<'b, S, T, I>(iter: I) -> Product<S>
where
    T: Tree<Symbol = S> + 'b,
    S: Symbol,
    I: IntoIterator<Item = &'b T>,
{
    iter.into_iter()
        .map(|tree| tree.into_product())
        .fold(Product::and_identity(), BitAnd::bitand)
}

pub fn visit_any<'b, S, T, I>(iter: I) -> Product<S>
where
    T: Tree<Symbol = S> + 'b,
    S: Symbol,
    I: IntoIterator<Item = &'b T>,
{
    iter.into_iter()
        .map(|tree| tree.into_product())
        .fold(Product::or_identity(), |accum, elem| &accum | &elem)
}

pub trait Tree: Sized {
    type Symbol: Symbol;
    fn into_product(&self) -> Product<Self::Symbol>;
    fn symbol(symbol: Self::Symbol) -> Self;
    fn all(trees: Vec<Self>) -> Self;
    fn any(trees: Vec<Self>) -> Self;
}

/// # Returns `None` means false
fn sum_into_tree<T, S>(sum: Sum<S>) -> Option<T>
where
    T: Tree<Symbol = S>,
    S: Symbol,
{
    let mut symbols: Vec<_> = sum.into_iter().map(T::symbol).collect();
    match symbols.len() {
        0 => None,
        1 => Some(symbols.pop().unwrap()),
        _ => Some(T::any(symbols)),
    }
}

/// # Returns `None` means false
fn product_into_tree<T, S>(product: Product<S>) -> Option<T>
where
    T: Tree<Symbol = S>,
    S: Symbol,
{
    let mut sums = product
        .into_iter()
        .map(sum_into_tree)
        .collect::<Option<Vec<_>>>()?;
    match sums.len() {
        0 => Some(T::all(Vec::default())),
        1 => Some(sums.pop().unwrap()),
        _ => Some(T::all(sums)),
    }
}

pub fn minimize<'a, 'b, T, S, M>(trees: M) -> impl Iterator<Item = (S, Option<T>)>
where
    'b: 'a,
    T: Tree<Symbol = S> + 'b,
    S: Symbol,
    M: IntoIterator<Item = (S, &'a T)>,
{
    let products = trees
        .into_iter()
        .map(|(symbol, tree)| (symbol, tree.into_product()))
        .collect();
    let mut products = Products { products };
    let len_before = products.len();
    products.minimize();
    eprintln!("Before: {}, After: {}", len_before, products.len());
    products
        .products
        .into_iter()
        .map(move |(symbol, product)| (symbol, product_into_tree(product)))
}

#[cfg(test)]
mod implications {
    use super::Product;
    use super::Products;
    use super::Sum;
    use super::Symbol;
    use std::cmp::Ordering;

    #[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Clone)]
    pub struct TestSymbol(u32);

    impl Symbol for TestSymbol {
        fn cmp_rank(&self, _other: &Self) -> Option<Ordering> {
            None
        }
    }

    #[test]
    fn foo() {
        let implications =
            Products::from([(TestSymbol(0), Product::from([Sum::from([TestSymbol(1)])]))]);
        assert!(implications.implies_test(&Sum::from([TestSymbol(0)]), &Sum::from([TestSymbol(1)])));
        assert!(implications.implies_test(&Sum::from([TestSymbol(0)]), &Sum::from([TestSymbol(0)])));
        assert!(implications.implies_test(&Sum::from([TestSymbol(1)]), &Sum::from([TestSymbol(1)])));
    }

    #[test]
    fn bar() {
        let implications = Products::from([
            (TestSymbol(0), Product::from([Sum::from([TestSymbol(1)])])),
            (TestSymbol(1), Product::from([Sum::from([TestSymbol(2)])])),
            (TestSymbol(2), Product::from([Sum::from([TestSymbol(3)])])),
            (TestSymbol(3), Product::from([Sum::from([TestSymbol(4)])])),
            (TestSymbol(4), Product::from([Sum::from([TestSymbol(5)])])),
        ]);
        assert!(implications.implies_test(&Sum::from([TestSymbol(0)]), &Sum::from([TestSymbol(0)])));
        assert!(implications.implies_test(&Sum::from([TestSymbol(0)]), &Sum::from([TestSymbol(1)])));
        assert!(implications.implies_test(&Sum::from([TestSymbol(1)]), &Sum::from([TestSymbol(2)])));
        assert!(implications.implies_test(&Sum::from([TestSymbol(0)]), &Sum::from([TestSymbol(5)])));

        assert!(
            !implications.implies_test(&Sum::from([TestSymbol(1)]), &Sum::from([TestSymbol(0)]))
        );
        assert!(
            !implications.implies_test(&Sum::from([TestSymbol(5)]), &Sum::from([TestSymbol(0)]))
        );
    }

    #[test]
    fn baz() {
        let implications = Products::from([
            (
                TestSymbol(0),
                Product::from([Sum::from([TestSymbol(1), TestSymbol(2)])]),
            ),
            (TestSymbol(1), Product::from([Sum::from([TestSymbol(3)])])),
            (TestSymbol(2), Product::from([Sum::from([TestSymbol(3)])])),
        ]);
        assert!(implications.implies_test(&Sum::from([TestSymbol(0)]), &Sum::from([TestSymbol(3)])));
        assert!(implications.implies_test(&Sum::from([TestSymbol(1)]), &Sum::from([TestSymbol(3)])));
        assert!(implications.implies_test(&Sum::from([TestSymbol(2)]), &Sum::from([TestSymbol(3)])));

        assert!(
            !implications.implies_test(&Sum::from([TestSymbol(0)]), &Sum::from([TestSymbol(1)]))
        );
        assert!(
            !implications.implies_test(&Sum::from([TestSymbol(3)]), &Sum::from([TestSymbol(0)]))
        );
    }

    #[test]
    fn qux() {
        let implications = Products::from([
            (
                TestSymbol(0),
                Product::from([Sum::from([TestSymbol(1), TestSymbol(2)])]),
            ),
            (
                TestSymbol(1),
                Product::from([Sum::from([TestSymbol(2), TestSymbol(3), TestSymbol(4)])]),
            ),
            (TestSymbol(2), Product::from([Sum::from([TestSymbol(5)])])),
            (TestSymbol(3), Product::from([Sum::from([TestSymbol(5)])])),
            (TestSymbol(4), Product::from([Sum::from([TestSymbol(5)])])),
        ]);
        assert!(implications.implies_test(&Sum::from([TestSymbol(0)]), &Sum::from([TestSymbol(5)])));

        assert!(
            !implications.implies_test(&Sum::from([TestSymbol(2)]), &Sum::from([TestSymbol(3)]))
        );
    }

    #[test]
    fn quoo() {
        let implications = Products::from([
            (TestSymbol(0), Product::from([Sum::from([TestSymbol(1)])])),
            (TestSymbol(1), Product::from([Sum::from([TestSymbol(2)])])),
            (TestSymbol(2), Product::from([Sum::from([TestSymbol(0)])])),
        ]);
        assert!(implications.implies_test(&Sum::from([TestSymbol(0)]), &Sum::from([TestSymbol(1)])));

        assert!(
            !implications.implies_test(&Sum::from([TestSymbol(0)]), &Sum::from([TestSymbol(3)]))
        );
    }

    #[test]
    fn quoo1() {
        let implications = Products::from([
            (TestSymbol(0), Product::from([Sum::from([TestSymbol(1)])])),
            (TestSymbol(1), Product::from([Sum::from([TestSymbol(2)])])),
            (
                TestSymbol(2),
                Product::from([Sum::from([TestSymbol(3)]), Sum::from([TestSymbol(0)])]),
            ),
        ]);
        assert!(implications.implies_test(&Sum::from([TestSymbol(0)]), &Sum::from([TestSymbol(3)])));
        assert!(implications.implies_test(&Sum::from([TestSymbol(0)]), &Sum::from([TestSymbol(1)])));

        assert!(
            !implications.implies_test(&Sum::from([TestSymbol(3)]), &Sum::from([TestSymbol(0)]))
        );
    }

    #[test]
    fn quoo2() {
        let implications = Products::from([
            (TestSymbol(0), Product::from([Sum::from([TestSymbol(1)])])),
            (TestSymbol(1), Product::from([Sum::from([TestSymbol(2)])])),
            (
                TestSymbol(2),
                Product::from([Sum::from([TestSymbol(0)]), Sum::from([TestSymbol(3)])]),
            ),
        ]);
        assert!(implications.implies_test(&Sum::from([TestSymbol(0)]), &Sum::from([TestSymbol(3)])));
        assert!(implications.implies_test(&Sum::from([TestSymbol(0)]), &Sum::from([TestSymbol(1)])));

        assert!(
            !implications.implies_test(&Sum::from([TestSymbol(3)]), &Sum::from([TestSymbol(0)]))
        );
    }
}
