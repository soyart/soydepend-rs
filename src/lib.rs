#![allow(dead_code)]

use std::collections::{HashMap, HashSet};

type Edges<T> = HashMap<T, HashSet<T>>;

#[derive(Default, Debug)]
pub struct Graph<T>
where
    T: Clone + Eq + std::hash::Hash,
{
    pub(crate) nodes: HashSet<T>,
    pub(crate) dependents: Edges<T>,
    pub(crate) dependencies: Edges<T>,
}

#[derive(Debug)]
pub enum Error {
    CircularDependency,
    DependencyExists,
    DependsOnSelf,
    NoSuchDirectDependency,
    NoSuchNode,
}

impl<T> Graph<T>
where
    T: Clone + Eq + std::hash::Hash,
{
    pub fn new() -> Self {
        Self {
            nodes: HashSet::default(),
            dependents: HashMap::default(),
            dependencies: HashMap::default(),
        }
    }

    /// Add dependency edges to the graph
    pub fn depend(&mut self, dependent: T, dependency: T) -> Result<(), Error> {
        if dependent == dependency {
            return Err(Error::DependsOnSelf);
        }

        if self.depends_on(&dependency, &dependent) {
            return Err(Error::CircularDependency);
        }

        self.nodes.insert(dependent.clone());
        self.nodes.insert(dependency.clone());

        insert_to_deps(&mut self.dependents, dependency.clone(), dependent.clone());
        insert_to_deps(&mut self.dependencies, dependent, dependency);

        Ok(())
    }

    /// Removes dependency edges from the graph
    pub fn undepend(&mut self, dependent: &T, dependency: &T) -> Result<(), Error> {
        if !self.depends_on_directly(dependent, dependency) {
            return Err(Error::NoSuchDirectDependency);
        }

        rm_from_deps(&mut self.dependencies, dependent, dependency);
        rm_from_deps(&mut self.dependents, dependency, dependent);

        Ok(())
    }

    #[inline(always)]
    pub fn contains(&self, node: &T) -> bool {
        self.nodes.contains(node)
    }

    /// Returns whether dependent depends directly on dependency
    #[inline(always)]
    pub fn depends_on_directly(&self, dependent: &T, dependency: &T) -> bool {
        self.dependencies
            .get(dependent)
            .map(|deps| deps.contains(dependency))
            .unwrap_or(false)
    }

    /// Returns deep dependencies of node
    pub fn dependencies(&self, node: &T) -> HashSet<T> {
        dig_deep(&self.dependencies, node)
    }

    /// Returns deep dependents of node
    pub fn dependents(&self, node: &T) -> HashSet<T> {
        dig_deep(&self.dependents, node)
    }

    /// Returns whether dependent depends on dependency in some way
    pub fn depends_on(&self, dependent: &T, dependency: &T) -> bool {
        self.dependencies(dependent).contains(dependency)
    }

    /// Returns whether the node is depended on by other
    pub fn is_dependend(&self, node: &T) -> bool {
        self.dependents
            .get(node)
            .is_some_and(|deps| !deps.is_empty())
    }

    /// Internal method for complete removal of the target
    fn delete(&mut self, target: &T) {
        if let Some(dependencies) = self.dependencies.get(target) {
            dependencies
                .iter()
                .for_each(|dependency| rm_from_deps(&mut self.dependents, dependency, target));
        }

        if let Some(dependents) = self.dependents.get(target) {
            dependents
                .iter()
                .for_each(|dependent| rm_from_deps(&mut self.dependencies, target, dependent));
        }

        self.dependencies.remove(target);
        self.dependents.remove(target);
        self.nodes.remove(target);
    }

    /// Removes undepended target node
    pub fn remove(&mut self, target: &T) -> Result<(), Error> {
        if !self.contains(target) {
            return Err(Error::NoSuchNode);
        }

        if self.is_dependend(target) {
            return Err(Error::DependencyExists);
        }

        self.delete(target);
        Ok(())
    }
}

fn insert_to_deps<T>(edges: &mut HashMap<T, HashSet<T>>, key: T, value: T)
where
    T: Clone + Eq + std::hash::Hash,
{
    match edges.get_mut(&key) {
        Some(set) => {
            set.insert(value);
        }
        None => {
            edges.insert(key, HashSet::from([value]));
        }
    };
}

#[inline(always)]
fn dig_deep<T>(edges: &HashMap<T, HashSet<T>>, node: &T) -> HashSet<T>
where
    T: Clone + Eq + std::hash::Hash,
{
    let mut search_next = vec![node];
    let mut result = HashSet::<T>::new();

    while !search_next.is_empty() {
        let mut discovered = Vec::new();

        for next in search_next.iter() {
            let nodes = edges.get(next);
            if nodes.is_none() {
                continue;
            }

            for n in nodes.unwrap() {
                if result.contains(n) {
                    continue;
                }

                discovered.push(n);
                result.insert(n.clone());
            }
        }

        search_next = discovered;
    }

    result
}

fn rm_from_deps<T>(edges: &mut Edges<T>, key: &T, target: &T)
where
    T: Clone + Eq + std::hash::Hash,
{
    let nodes = edges.get_mut(key);
    if nodes.is_none() {
        return;
    }

    let nodes = nodes.unwrap();
    if !nodes.contains(target) {
        return;
    }

    if nodes.len() <= 1 {
        edges.remove(key);
        return;
    }

    nodes.remove(target);
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CircularDependency => write!(f, "circular dependency"),
            Self::DependencyExists => write!(f, "dependencies exist"),
            Self::DependsOnSelf => write!(f, "depends on self"),
            Self::NoSuchDirectDependency => write!(f, "no such direct dependency relationship"),
            Self::NoSuchNode => write!(f, "no such node"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BIGBANG: &'static str = "bigbang";
    const STARDUST: &'static str = "stardust";
    const STAR: &'static str = "star";
    const PROTO_PLANET: &'static str = "proto-planet";
    const PLANET: &'static str = "planet";

    fn default_graph<'a>() -> Graph<&'a str> {
        let mut g = Graph::<&str>::default();
        g.depend(STARDUST, BIGBANG).unwrap();
        g.depend(STAR, STARDUST).unwrap();
        g.depend(PROTO_PLANET, STAR).unwrap();
        g.depend(PLANET, PROTO_PLANET).unwrap();

        g
    }

    #[test]
    fn test_basic_dependency() {
        let mut g = default_graph();
        assert_eq!(g.dependents(&STAR), HashSet::from([PROTO_PLANET, PLANET]));

        assert!(g.depends_on(&STAR, &STARDUST));
        assert!(g.depends_on(&STAR, &BIGBANG));
        assert!(g.depends_on(&STARDUST, &BIGBANG));

        assert!(g.depends_on(&PLANET, &BIGBANG));
        assert!(g.depends_on(&PLANET, &STARDUST));
        assert!(g.depends_on(&PLANET, &STAR));
        assert!(g.depends_on(&PLANET, &PROTO_PLANET));

        assert!(g.depends_on(&PROTO_PLANET, &BIGBANG));
        assert!(g.depends_on(&PROTO_PLANET, &STARDUST));
        assert!(g.depends_on(&PROTO_PLANET, &STAR));

        assert!(g.depends_on(&STAR, &BIGBANG));
        assert!(g.depends_on(&STAR, &STARDUST));

        assert!(g.depends_on(&STARDUST, &BIGBANG));

        assert!(!g.depends_on(&BIGBANG, &STARDUST));
        assert!(!g.depends_on(&BIGBANG, &PLANET));
        assert!(!g.depends_on(&STARDUST, &PLANET));
        assert!(!g.depends_on(&STAR, &PLANET));
        assert!(!g.depends_on(&PROTO_PLANET, &PLANET));
        assert!(!g.depends_on(&PLANET, &PLANET));

        assert!(!g.depends_on(&BIGBANG, &PROTO_PLANET));
        assert!(!g.depends_on(&STARDUST, &PROTO_PLANET));
        assert!(!g.depends_on(&STAR, &PROTO_PLANET));
        assert!(!g.depends_on(&PROTO_PLANET, &PROTO_PLANET));

        assert!(!g.depends_on(&BIGBANG, &STAR));
        assert!(!g.depends_on(&STARDUST, &STAR));
        assert!(!g.depends_on(&STAR, &STAR));

        assert!(!g.depends_on(&BIGBANG, &STARDUST));
        assert!(!g.depends_on(&STARDUST, &STARDUST));

        g.depend(STARDUST, STAR)
            .expect_err("stardust should not depend on star");

        g.depend(BIGBANG, "god").unwrap();
        g.depend("sun", STARDUST).unwrap();
        g.depend("earth", "sun").unwrap();
        g.depend("human", "earth").unwrap();

        assert!(g.depends_on(&"human", &"earth"));
        assert!(g.depends_on(&"human", &"sun"));
        assert!(g.depends_on(&"human", &STARDUST));
        assert!(g.depends_on(&"human", &"god"));
    }

    #[test]
    fn test_depends_on_directly() {
        let mut g = Graph::new();

        g.depend("b", "a").unwrap();
        g.depend("c", "b").unwrap();
        g.depend("d", "c").unwrap();

        assert!(g.depends_on_directly(&"d", &"c"));
        assert!(g.depends_on_directly(&"c", &"b"));
        assert!(g.depends_on_directly(&"b", &"a"));

        assert!(!g.depends_on_directly(&"d", &"b"));
        assert!(!g.depends_on_directly(&"c", &"a"));
        assert!(!g.depends_on_directly(&"b", &"x"));
    }

    #[test]
    fn test_deep_dig() {
        let mut g = default_graph();

        assert_eq!(
            g.dependents(&BIGBANG), //
            HashSet::from([STAR, STARDUST, PROTO_PLANET, PLANET])
        );

        assert_eq!(
            g.dependencies(&BIGBANG), //
            HashSet::default(),
        );

        assert_eq!(
            g.dependencies(&STARDUST), //
            HashSet::from([BIGBANG]),
        );

        assert_eq!(
            g.dependencies(&STAR), //
            HashSet::from([BIGBANG, STARDUST]),
        );

        assert_eq!(
            g.dependencies(&PLANET), //
            HashSet::from([BIGBANG, STARDUST, STAR, PROTO_PLANET]),
        );

        g.depend(BIGBANG, "god").unwrap();
        g.depend("sun", STARDUST).unwrap();
        g.depend("earth", "sun").unwrap();
        g.depend("earth", "god").unwrap();
        g.depend("human", "earth").unwrap();

        {
            assert_eq!(
                g.dependents(&"god"),
                HashSet::from([
                    BIGBANG,
                    STARDUST,
                    STAR,
                    PROTO_PLANET,
                    PLANET,
                    "sun",
                    "earth",
                    "human"
                ]),
            );

            assert_eq!(
                g.dependents(&BIGBANG),
                HashSet::from([
                    STARDUST,
                    STAR,
                    PROTO_PLANET,
                    PLANET,
                    "sun",
                    "earth",
                    "human"
                ])
            );

            assert_eq!(
                g.dependents(&STARDUST),
                HashSet::from([STAR, PROTO_PLANET, PLANET, "sun", "earth", "human"])
            );

            assert_eq!(g.dependents(&STAR), HashSet::from([PROTO_PLANET, PLANET]));
        }

        {
            assert_eq!(g.dependencies(&"god"), HashSet::default());

            assert_eq!(
                g.dependencies(&"sun"),
                HashSet::from([STARDUST, BIGBANG, "god"])
            );

            assert_eq!(
                g.dependencies(&"earth"),
                HashSet::from([STARDUST, BIGBANG, "god", "sun"])
            );

            assert_eq!(
                g.dependencies(&"human"),
                HashSet::from(["god", BIGBANG, STARDUST, "sun", "earth"])
            );

            g.depend("human", PLANET).unwrap();
            assert_eq!(
                g.dependencies(&"human"),
                HashSet::from([
                    "god",
                    BIGBANG,
                    STARDUST,
                    "sun",
                    "earth",
                    STAR,
                    PROTO_PLANET,
                    PLANET
                ])
            );
        }
    }

    #[test]
    fn test_undepend() {
        let mut g = Graph::<&str>::default();
        g.depend(STARDUST, BIGBANG).unwrap();
        g.depend(STAR, STARDUST).unwrap();

        g.undepend(&STAR, &BIGBANG)
            .expect_err("should not be able to undepend deep dependency");

        g.undepend(&STAR, &STARDUST)
            .expect("should be able to undepend direct dependency");

        assert!(!g.depends_on(&STAR, &STARDUST));
        assert!(!g.depends_on(&STAR, &BIGBANG));
    }

    #[test]
    fn test_remove() {
        let mut g = default_graph();

        g.remove(&PROTO_PLANET)
            .expect_err("proto-planet is depended on by planet");

        g.remove(&PLANET).unwrap();

        assert_eq!(
            g.dependents(&BIGBANG),
            HashSet::from([STARDUST, STAR, PROTO_PLANET,])
        );

        assert!(!g.contains(&PLANET));
        assert_eq!(g.dependencies(&PLANET), HashSet::default());
        assert_eq!(g.dependents(&PLANET), HashSet::default());

        assert_eq!(g.dependents(&PROTO_PLANET), HashSet::default());
        assert_eq!(g.dependents(&STAR), HashSet::from([PROTO_PLANET]));
        assert_eq!(g.dependents(&STARDUST), HashSet::from([STAR, PROTO_PLANET]));
        assert_eq!(
            g.dependents(&BIGBANG),
            HashSet::from([STARDUST, STAR, PROTO_PLANET])
        );

        assert_eq!(
            g.dependencies(&PROTO_PLANET),
            HashSet::from([STAR, STARDUST, BIGBANG])
        );
        assert_eq!(g.dependencies(&STAR), HashSet::from([STARDUST, BIGBANG]));
        assert_eq!(g.dependencies(&STARDUST), HashSet::from([BIGBANG]));
        assert_eq!(g.dependencies(&BIGBANG), HashSet::default());

        g.remove(&PROTO_PLANET).unwrap();

        assert!(!g.contains(&PROTO_PLANET));
        assert_eq!(g.dependencies(&PROTO_PLANET), HashSet::default());
        assert_eq!(g.dependents(&PROTO_PLANET), HashSet::default());

        assert_eq!(g.dependents(&STAR), HashSet::default());
        assert_eq!(g.dependents(&STARDUST), HashSet::from([STAR]));
        assert_eq!(g.dependents(&BIGBANG), HashSet::from([STARDUST, STAR]));

        assert_eq!(g.dependencies(&STAR), HashSet::from([STARDUST, BIGBANG]));
        assert_eq!(g.dependencies(&STARDUST), HashSet::from([BIGBANG]));
        assert_eq!(g.dependencies(&BIGBANG), HashSet::default());
    }
}
